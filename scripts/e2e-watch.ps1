[CmdletBinding()]
param(
    [switch]$Once
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$pointer = Join-Path $root '.agent-runs/e2e/latest.log.path'

if (-not (Test-Path -LiteralPath $pointer -PathType Leaf)) {
    throw "[e2e-watch] no active or previous run: $pointer is missing"
}

$logFile = (Get-Content -LiteralPath $pointer -Raw).Trim()
if ($logFile -match '^/mnt/([a-zA-Z])/(.+)$') {
    $drive = $Matches[1].ToUpperInvariant()
    $relative = $Matches[2] -replace '/', '\'
    $logFile = "${drive}:\$relative"
}
$statusFile = Join-Path (Split-Path -Parent $logFile) 'status.json'

Write-Host "[e2e-watch] log: $logFile"
Write-Host "[e2e-watch] status: $statusFile"
if (Test-Path -LiteralPath $statusFile -PathType Leaf) {
    Get-Content -LiteralPath $statusFile
}

if ($Once) {
    Get-Content -LiteralPath $logFile -Tail 100
} else {
    Get-Content -LiteralPath $logFile -Tail 100 -Wait
}
