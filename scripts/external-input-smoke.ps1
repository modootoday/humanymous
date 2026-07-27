param(
    [ValidateSet("chromium", "firefox")]
    [string]$Browser = "chromium",
    [switch]$Hil,
    [switch]$Keep,
    [switch]$NoBuild,
    [string]$RunId = ""
)

$ErrorActionPreference = "Stop"
$arguments = @("scripts/external-input-smoke.mjs", "--browser", $Browser)
if ($Hil) { $arguments += "--hil" }
if ($Keep) { $arguments += "--keep" }
if ($NoBuild) { $arguments += "--no-build" }
if ($RunId) { $arguments += @("--run-id", $RunId) }

& node @arguments
exit $LASTEXITCODE
