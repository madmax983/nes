$ErrorActionPreference = "Stop"

$verus = "C:\Users\markm\verus\verus.exe"
if (-not (Test-Path $verus)) {
    $verus = "verus"
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$proofFiles = @(
    (Join-Path $repoRoot "crates/nes-proof/src/cpu_model.rs"),
    (Join-Path $repoRoot "crates/nes-proof/src/status_flags.rs"),
    (Join-Path $repoRoot "crates/nes-proof/src/bus_map.rs"),
    (Join-Path $repoRoot "crates/nes-proof/src/mapper_nrom_uxrom.rs")
)

foreach ($file in $proofFiles) {
    Write-Host "[verus] checking $file"
    & $verus $file
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

Write-Host "All Verus checks passed."
