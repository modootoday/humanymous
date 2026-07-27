param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("reference-relative-v1")]
    [string]$Model,

    [ValidateSet("3v", "4v")]
    [string]$Mode = "3v",

    [ValidateSet("chromium", "firefox")]
    [string]$Browser = "chromium",

    [ValidatePattern('^[A-Za-z0-9._:-]{1,128}$')]
    [string]$StrategySeed,

    [switch]$NoBuild
)

$ErrorActionPreference = "Stop"

$arguments = @(
    "scripts/external-input-kernel-runner.mjs",
    "--model", $Model,
    "--browser", $Browser,
    "--mode", $Mode
)
if ($StrategySeed) {
    $arguments += @("--strategy-seed", $StrategySeed)
}
if ($NoBuild) {
    $arguments += "--no-build"
}

& node @arguments
exit $LASTEXITCODE
