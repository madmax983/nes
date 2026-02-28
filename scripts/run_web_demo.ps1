param(
    [int]$Port = 8080,
    [switch]$OpenBrowser,
    [switch]$SkipHomebrew,
    [switch]$Dev
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$webCrate = Join-Path $repoRoot "crates\nes-web"
$webPkg = Join-Path $repoRoot "web\pkg"
$pythonServer = Join-Path $repoRoot "scripts\web_server.py"
$url = "http://127.0.0.1:$Port/web/index.html"

if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    throw "wasm-pack is required. Install via: cargo install wasm-pack"
}
$pythonCmd = Get-Command py -ErrorAction SilentlyContinue
if (-not $pythonCmd) {
    $pythonCmd = Get-Command python -ErrorAction SilentlyContinue
}
$nodeCmd = Get-Command node -ErrorAction SilentlyContinue
if (-not $pythonCmd -and -not $nodeCmd) {
    throw "Need Python (py/python) or node for local static hosting."
}

Push-Location $repoRoot
try {
    if (-not $SkipHomebrew) {
        cargo run -p nes-test-harness --bin build_homebrew_rom
    }

    Push-Location $webCrate
    try {
        if ($Dev) {
            wasm-pack build --target web --out-dir ../../web/pkg --dev
        } else {
            wasm-pack build --target web --out-dir ../../web/pkg
        }
    }
    finally {
        Pop-Location
    }

    Write-Host "Web package ready: $webPkg"
    Write-Host "Serving: $url"
    if ($OpenBrowser) {
        Start-Process $url | Out-Null
    }

    if ($pythonCmd) {
        & $pythonCmd.Source $pythonServer --port $Port --root $repoRoot
    }
    else {
        node .\web\server.mjs $Port
    }
}
finally {
    Pop-Location
}
