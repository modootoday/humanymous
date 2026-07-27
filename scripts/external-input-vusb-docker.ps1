param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-z][a-z0-9-]{2,63}$')]
    [string]$Model,

    [ValidateSet("auto", "native", "kernel")]
    [string]$Runner = "auto",

    [ValidateSet("3v", "4v")]
    [string]$Mode = "3v",

    [ValidateSet("chromium", "firefox")]
    [string]$Browser = "chromium",

    [ValidatePattern('^[A-Za-z0-9._:-]{1,128}$')]
    [string]$StrategySeed,

    [switch]$NoBuild
)

$ErrorActionPreference = "Stop"

$useKernel = $Runner -eq "kernel" -or -not $IsLinux
if ($Runner -eq "auto" -and $IsLinux) {
    $dockerOperatingSystem = & docker info --format '{{.OperatingSystem}}'
    if ($LASTEXITCODE -ne 0) {
        throw "Docker daemon inspection failed"
    }
    $useKernel = $dockerOperatingSystem -like "*Docker Desktop*"
}

if ($useKernel) {
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
}

if ($Runner -eq "native" -and -not $IsLinux) {
    [Console]::Error.WriteLine(
        '{"status":"UNAVAILABLE","capability":"native-linux-vm","reason":"native virtual USB requires Linux; select the Docker kernel runner"}'
    )
    exit 3
}

$bash = Get-Command bash -ErrorAction SilentlyContinue
if (-not $bash) {
    [Console]::Error.WriteLine(
        '{"status":"UNAVAILABLE","capability":"bash-supervisor","reason":"canonical Bash supervisor is unavailable"}'
    )
    exit 3
}

& $bash.Source "scripts/external-input-vusb-docker.sh" "--model" $Model
exit $LASTEXITCODE
