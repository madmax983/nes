param(
    [string]$BindAddr = '127.0.0.1:6502',
    [string]$CaptureDir = '',
    [int]$SegmentRetries = 4,
    [switch]$KeepWindowOpen
)

$ErrorActionPreference = 'Stop'

if ($CaptureDir -eq '') {
    $CaptureDir = Join-Path (Get-Location) 'target\hybrid_captures'
}
New-Item -ItemType Directory -Path $CaptureDir -Force | Out-Null

$stdoutLog = Join-Path (Get-Location) 'target\hybrid_autoplay_out.log'
$stderrLog = Join-Path (Get-Location) 'target\hybrid_autoplay_err.log'
Remove-Item $stdoutLog, $stderrLog -ErrorAction SilentlyContinue

$desktopExe = Join-Path (Get-Location) 'target\debug\nes-desktop.exe'
if (-not (Test-Path $desktopExe)) {
    throw "Missing desktop binary at $desktopExe. Build first with: cargo build -p nes-desktop --features mcp-host"
}

function Split-BindAddr {
    param([string]$Value)
    $parts = $Value.Split(':')
    if ($parts.Count -lt 2) {
        throw "Invalid bind address '$Value' (expected host:port)."
    }
    $port = 0
    if (-not [int]::TryParse($parts[-1], [ref]$port)) {
        throw "Invalid bind address '$Value' (port must be an integer)."
    }
    $bindHost = ($parts[0..($parts.Count - 2)] -join ':')
    if ($bindHost -eq '') {
        throw "Invalid bind address '$Value' (missing host)."
    }
    return @($bindHost, $port)
}

function Read-McpResponse {
    param([System.IO.StreamReader]$Reader)

    $contentLength = $null
    while ($true) {
        $line = $Reader.ReadLine()
        if ($null -eq $line) { throw 'EOF while reading MCP headers' }
        if ($line -eq '') { break }
        if ($line.StartsWith('Content-Length:')) {
            $contentLength = [int]$line.Substring(15).Trim()
        }
    }
    if ($null -eq $contentLength) {
        throw 'Missing Content-Length header'
    }

    $buffer = New-Object char[] $contentLength
    $read = 0
    while ($read -lt $contentLength) {
        $n = $Reader.Read($buffer, $read, $contentLength - $read)
        if ($n -le 0) { throw 'EOF while reading MCP payload' }
        $read += $n
    }
    $json = -join $buffer
    return ($json | ConvertFrom-Json)
}

function Send-McpRequest {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [object]$Request
    )

    $body = $Request | ConvertTo-Json -Compress -Depth 16
    $bodyBytes = [System.Text.Encoding]::UTF8.GetBytes($body)
    $headerBytes = [System.Text.Encoding]::ASCII.GetBytes("Content-Length: $($bodyBytes.Length)`r`n`r`n")

    $Stream.Write($headerBytes, 0, $headerBytes.Length)
    $Stream.Write($bodyBytes, 0, $bodyBytes.Length)
    $Stream.Flush()

    return Read-McpResponse -Reader $Reader
}

function Invoke-McpTool {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [string]$Name,
        [hashtable]$Arguments = @{}
    )
    $request = @{
        jsonrpc = '2.0'
        id = $Id.Value
        method = 'tools/call'
        params = @{ name = $Name; arguments = $Arguments }
    }
    $Id.Value = $Id.Value + 1
    return Send-McpRequest -Stream $Stream -Reader $Reader -Request $request
}

function Invoke-McpToolChecked {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [string]$Name,
        [hashtable]$Arguments = @{}
    )
    $resp = Invoke-McpTool -Stream $Stream -Reader $Reader -Id $Id -Name $Name -Arguments $Arguments
    if ($resp.result.isError) {
        $message = $resp.result.content[0].text
        throw "Tool '$Name' failed: $message"
    }
    return $resp.result.structuredContent
}

function New-ButtonState {
    return @{
        A = $false
        B = $false
        Select = $false
        Start = $false
        Up = $false
        Down = $false
        Left = $false
        Right = $false
    }
}

function Set-Button {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [hashtable]$State,
        [string]$Button,
        [bool]$Pressed
    )
    if ($State[$Button] -eq $Pressed) {
        return
    }
    if ($Pressed) {
        [void](Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'press_button' -Arguments @{ button = $Button })
    }
    else {
        [void](Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'release_button' -Arguments @{ button = $Button })
    }
    $State[$Button] = $Pressed
}

function Set-DriveButtons {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [hashtable]$State,
        [bool]$Right,
        [bool]$B
    )
    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button 'Right' -Pressed $Right
    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button 'B' -Pressed $B
}

function Reset-Buttons {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [hashtable]$State
    )
    foreach ($button in @('A', 'B', 'Select', 'Start', 'Up', 'Down', 'Left', 'Right')) {
        Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button $button -Pressed $false
    }
}

function Invoke-StartSequence {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [hashtable]$State
    )
    Start-Sleep -Seconds 2

    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button 'Start' -Pressed $true
    Start-Sleep -Milliseconds 220
    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button 'Start' -Pressed $false

    Start-Sleep -Milliseconds 700

    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button 'Start' -Pressed $true
    Start-Sleep -Milliseconds 220
    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $State -Button 'Start' -Pressed $false

    Start-Sleep -Seconds 3
}

function Shift-JumpSchedule {
    param([int[]]$JumpMs, [int]$Attempt)
    $offsets = @(0, -80, 80, -140, 140, -200, 200)
    $index = [Math]::Min($Attempt, $offsets.Count - 1)
    $offset = $offsets[$index]
    $shifted = @()
    foreach ($ms in $JumpMs) {
        $value = $ms + $offset
        if ($value -lt 0) { $value = 0 }
        $shifted += $value
    }
    return ,$shifted
}

function Invoke-SegmentAttempt {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [hashtable]$ButtonState,
        [pscustomobject]$Segment,
        [int]$Attempt
    )
    $jumpMs = Shift-JumpSchedule -JumpMs $Segment.JumpsMs -Attempt $Attempt
    Set-DriveButtons -Stream $Stream -Reader $Reader -Id $Id -State $ButtonState -Right $Segment.HoldRight -B $Segment.HoldB
    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $ButtonState -Button 'A' -Pressed $false

    $jumpIndex = 0
    $jumpReleaseAt = -1
    $lastSampleMs = -9999
    $sampleIntervalMs = 110
    $pcStallCount = 0
    $lastPc = -1
    $failure = $null

    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $Segment.DurationMs) {
        $elapsedMs = [int]$sw.ElapsedMilliseconds

        if ($jumpReleaseAt -ge 0 -and $elapsedMs -ge $jumpReleaseAt) {
            Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $ButtonState -Button 'A' -Pressed $false
            $jumpReleaseAt = -1
        }

        if ($jumpIndex -lt $jumpMs.Count -and $elapsedMs -ge $jumpMs[$jumpIndex]) {
            Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $ButtonState -Button 'A' -Pressed $true
            $jumpReleaseAt = $elapsedMs + 95
            $jumpIndex++
        }

        if (($elapsedMs - $lastSampleMs) -ge $sampleIntervalMs) {
            $regs = Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'read_registers'
            $pc = [int]$regs.pc
            if ($pc -eq $lastPc) {
                $pcStallCount++
            }
            else {
                $pcStallCount = 0
                $lastPc = $pc
            }
            if ($pcStallCount -ge 22) {
                $failure = "pc_stall"
                break
            }
            $lastSampleMs = $elapsedMs
        }

        Start-Sleep -Milliseconds 24
    }

    Set-Button -Stream $Stream -Reader $Reader -Id $Id -State $ButtonState -Button 'A' -Pressed $false
    $regsEnd = Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'read_registers'
    $ppuEnd = Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'get_ppu_frame_counter'
    $stateEnd = Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'get_emulator_state'

    if ($null -eq $failure) {
        return @{
            success = $true
            reason = ''
            regs = $regsEnd
            ppu = $ppuEnd
            state = $stateEnd
            jumps = $jumpMs
        }
    }

    return @{
        success = $false
        reason = $failure
        regs = $regsEnd
        ppu = $ppuEnd
        state = $stateEnd
        jumps = $jumpMs
    }
}

function Capture-SegmentFrame {
    param(
        [System.IO.Stream]$Stream,
        [System.IO.StreamReader]$Reader,
        [ref]$Id,
        [string]$CaptureDir,
        [string]$Name,
        [int]$Attempt
    )
    $safeName = $Name.Replace(' ', '_').ToLowerInvariant()
    $path = Join-Path $CaptureDir ("{0}_attempt{1:D2}.bmp" -f $safeName, $Attempt)
    [void](Invoke-McpToolChecked -Stream $Stream -Reader $Reader -Id $Id -Name 'capture_frame' -Arguments @{ path = $path })
    return $path
}

$desktop = Start-Process `
    -FilePath $desktopExe `
    -ArgumentList @('--mcp-host', '--mcp-bind', $BindAddr) `
    -PassThru `
    -RedirectStandardOutput $stdoutLog `
    -RedirectStandardError $stderrLog

$client = $null
$summary = @()
$capturePaths = @()
$buttonState = New-ButtonState

try {
    $hostInfo = Split-BindAddr -Value $BindAddr
    $bindHost = $hostInfo[0]
    $port = [int]$hostInfo[1]

    $deadline = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $deadline -and $null -eq $client) {
        try {
            $client = New-Object System.Net.Sockets.TcpClient($bindHost, $port)
        }
        catch {
            Start-Sleep -Milliseconds 200
        }
    }
    if ($null -eq $client) {
        throw "Timed out waiting for embedded MCP host at $BindAddr"
    }

    $stream = $client.GetStream()
    $reader = New-Object System.IO.StreamReader($stream, [System.Text.Encoding]::UTF8, $false, 1024, $true)
    $id = [ref]1

    $init = Send-McpRequest -Stream $stream -Reader $reader -Request @{
        jsonrpc = '2.0'
        id = $id.Value
        method = 'initialize'
        params = @{ protocolVersion = '2025-06-18'; capabilities = @{} }
    }
    $id.Value = $id.Value + 1
    [void](Send-McpRequest -Stream $stream -Reader $reader -Request @{ jsonrpc = '2.0'; id = $id.Value; method = 'tools/list'; params = @{} })
    $id.Value = $id.Value + 1

    Write-Output ("server=" + $init.result.serverInfo.name)
    Write-Output 'boot=starting-title-sequence'
    Invoke-StartSequence -Stream $stream -Reader $reader -Id $id -State $buttonState

    # Macro plan: segment timing with deterministic frame-level controller and retry hooks.
    $segments = @(
        [pscustomobject]@{ Name = 'opening_run'; DurationMs = 1200; HoldRight = $true; HoldB = $true; JumpsMs = @() },
        [pscustomobject]@{ Name = 'first_goomba'; DurationMs = 1000; HoldRight = $true; HoldB = $true; JumpsMs = @(560) },
        [pscustomobject]@{ Name = 'question_blocks'; DurationMs = 1200; HoldRight = $true; HoldB = $true; JumpsMs = @(420, 930) },
        [pscustomobject]@{ Name = 'pipe_zone'; DurationMs = 1500; HoldRight = $true; HoldB = $true; JumpsMs = @(500, 1060) }
    )

    for ($segIndex = 0; $segIndex -lt $segments.Count; $segIndex++) {
        $segment = $segments[$segIndex]
        $slot = ("hybrid_seg_{0:D2}" -f $segIndex)
        [void](Invoke-McpToolChecked -Stream $stream -Reader $reader -Id $id -Name 'save_state' -Arguments @{ slot = $slot })

        $succeeded = $false
        for ($attempt = 0; $attempt -lt $SegmentRetries; $attempt++) {
            if ($attempt -gt 0) {
                [void](Invoke-McpToolChecked -Stream $stream -Reader $reader -Id $id -Name 'load_state' -Arguments @{ slot = $slot })
                Set-DriveButtons -Stream $stream -Reader $reader -Id $id -State $buttonState -Right $false -B $false
                Set-Button -Stream $stream -Reader $reader -Id $id -State $buttonState -Button 'A' -Pressed $false
                Start-Sleep -Milliseconds 80
            }

            $result = Invoke-SegmentAttempt -Stream $stream -Reader $reader -Id $id -ButtonState $buttonState -Segment $segment -Attempt $attempt
            $capturePath = Capture-SegmentFrame -Stream $stream -Reader $reader -Id $id -CaptureDir $CaptureDir -Name $segment.Name -Attempt $attempt
            $capturePaths += $capturePath

            $summary += [pscustomobject]@{
                segment = $segment.Name
                attempt = $attempt
                success = $result.success
                reason = $result.reason
                pc = $result.regs.pc
                ppu_frame = $result.ppu.frame_counter
                jumps = ($result.jumps -join ',')
                capture = $capturePath
            }

            if ($result.success) {
                $succeeded = $true
                break
            }
        }

        if (-not $succeeded) {
            throw "Segment '$($segment.Name)' failed after $SegmentRetries attempts."
        }
    }

    Reset-Buttons -Stream $stream -Reader $reader -Id $id -State $buttonState
    $finalCapture = Capture-SegmentFrame -Stream $stream -Reader $reader -Id $id -CaptureDir $CaptureDir -Name 'final' -Attempt 0
    $capturePaths += $finalCapture
    $finalRegs = Invoke-McpToolChecked -Stream $stream -Reader $reader -Id $id -Name 'read_registers'
    $finalPpu = Invoke-McpToolChecked -Stream $stream -Reader $reader -Id $id -Name 'get_ppu_frame_counter'

    Write-Output ('result=success')
    Write-Output ("final_pc=$($finalRegs.pc)")
    Write-Output ("final_ppu_frame=$($finalPpu.frame_counter)")
}
finally {
    if ($null -ne $client) {
        $client.Close()
    }

    if ($summary.Count -gt 0) {
        Write-Output '--- segment summary ---'
        foreach ($row in $summary) {
            Write-Output ("segment={0} attempt={1} success={2} reason={3} pc={4} ppu={5} jumps=[{6}] capture={7}" -f `
                    $row.segment, $row.attempt, $row.success, $row.reason, $row.pc, $row.ppu_frame, $row.jumps, $row.capture)
        }
    }

    if ($capturePaths.Count -gt 0) {
        Write-Output '--- captures ---'
        foreach ($path in $capturePaths) {
            Write-Output $path
        }
    }

    if ($null -ne $desktop -and -not $desktop.HasExited -and -not $KeepWindowOpen) {
        Stop-Process -Id $desktop.Id -Force
    }

    Write-Output '--- desktop stdout ---'
    if (Test-Path $stdoutLog) {
        Get-Content $stdoutLog
    }
    Write-Output '--- desktop stderr ---'
    if (Test-Path $stderrLog) {
        Get-Content $stderrLog
    }
}
