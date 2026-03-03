$ErrorActionPreference = 'Stop'

$capture = Join-Path (Get-Location) 'target\mcp_play_demo.bmp'
$exe = Join-Path (Get-Location) 'target\debug\nes-desktop.exe'
$stdoutLog = Join-Path (Get-Location) 'target\desktop_run_out.log'
$stderrLog = Join-Path (Get-Location) 'target\desktop_run_err.log'

Remove-Item $stdoutLog, $stderrLog, $capture -ErrorAction SilentlyContinue

if (-not (Test-Path $exe)) {
    throw "Missing desktop binary at $exe. Build first with: cargo build -p nes-desktop --features mcp-host"
}

$desktop = Start-Process `
    -FilePath $exe `
    -ArgumentList @('--mcp-host', '--mcp-bind', '127.0.0.1:6502') `
    -PassThru `
    -RedirectStandardOutput $stdoutLog `
    -RedirectStandardError $stderrLog

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
    if ($null -eq $contentLength) { throw 'Missing Content-Length header' }

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
    param([System.IO.Stream]$Stream, [System.IO.StreamReader]$Reader, [hashtable]$Request)
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

$client = $null
try {
    $deadline = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $deadline -and $null -eq $client) {
        try {
            $client = New-Object System.Net.Sockets.TcpClient('127.0.0.1', 6502)
        } catch {
            Start-Sleep -Milliseconds 200
        }
    }
    if ($null -eq $client) {
        throw 'Timed out waiting for embedded MCP host at 127.0.0.1:6502'
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

    $null = Send-McpRequest -Stream $stream -Reader $reader -Request @{
        jsonrpc = '2.0'; id = $id.Value; method = 'tools/list'; params = @{}
    }
    $id.Value = $id.Value + 1

    Start-Sleep -Seconds 2

    # Start from title/menu.
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'press_button' -Arguments @{ button = 'Start' }
    Start-Sleep -Milliseconds 220
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'release_button' -Arguments @{ button = 'Start' }
    Start-Sleep -Milliseconds 700
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'press_button' -Arguments @{ button = 'Start' }
    Start-Sleep -Milliseconds 220
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'release_button' -Arguments @{ button = 'Start' }
    Start-Sleep -Seconds 3

    # Move right, run, and jump.
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'press_button' -Arguments @{ button = 'Right' }
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'press_button' -Arguments @{ button = 'B' }
    Start-Sleep -Milliseconds 900
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'press_button' -Arguments @{ button = 'A' }
    Start-Sleep -Milliseconds 120
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'release_button' -Arguments @{ button = 'A' }
    Start-Sleep -Seconds 2

    $captureResult = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'capture_frame' -Arguments @{ path = $capture }
    $regs = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'read_registers'
    $ppu = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'get_ppu_frame_counter'
    $state = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'get_emulator_state'

    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'release_button' -Arguments @{ button = 'Right' }
    $null = Invoke-McpTool -Stream $stream -Reader $reader -Id $id -Name 'release_button' -Arguments @{ button = 'B' }

    Write-Output ("init_server=" + $init.result.serverInfo.name)
    Write-Output ("capture_result=" + (($captureResult.result.structuredContent | ConvertTo-Json -Compress)))
    Write-Output ("registers=" + (($regs.result.structuredContent | ConvertTo-Json -Compress)))
    Write-Output ("ppu=" + (($ppu.result.structuredContent | ConvertTo-Json -Compress)))
    Write-Output ("state=" + (($state.result.structuredContent | ConvertTo-Json -Compress)))
    Write-Output ("capture_exists=" + (Test-Path $capture))
    Write-Output ("capture_path=$capture")
} finally {
    if ($null -ne $client) {
        $client.Close()
    }
    Start-Sleep -Seconds 1
    if ($null -ne $desktop -and -not $desktop.HasExited) {
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
