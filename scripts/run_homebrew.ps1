$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$romPath = Join-Path $repoRoot "roms\homebrew\homebrew.nes"

Push-Location $repoRoot
try {
    cargo run -p nes-test-harness --bin build_homebrew_rom -- --out $romPath
    cargo run -p nes-desktop --release -- $romPath
} finally {
    Pop-Location
}
