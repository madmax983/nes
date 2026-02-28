param(
    [int]$Port = 8080,
    [switch]$OpenBrowser,
    [switch]$SkipHomebrew,
    [switch]$Dev
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$webCrate = Join-Path $repoRoot "crates\nes-web"
$url = "http://127.0.0.1:$Port/"

if (-not (Get-Command trunk -ErrorAction SilentlyContinue)) {
    throw "trunk is required. Install via: cargo install trunk"
}

Push-Location $repoRoot
try {
    if (-not $SkipHomebrew) {
        cargo run -p nes-test-harness --bin build_homebrew_rom
    }

    Write-Host "Starting Trunk dev server..."

    $trunkArgs = @(
        "serve",
        "--config",
        "Trunk.toml",
        "--address",
        "127.0.0.1",
        "--port",
        "$Port"
    )
    if ($OpenBrowser) {
        $trunkArgs += "--open"
    }
    if (-not $Dev) {
        $trunkArgs += "--release"
    }

    Write-Host "Serving: $url"
    Push-Location $webCrate
    try {
        trunk @trunkArgs
    }
    finally {
        Pop-Location
    }
}
finally {
    Pop-Location
}
