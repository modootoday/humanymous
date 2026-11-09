#Requires -Version 5.1
<#
.SYNOPSIS
  Single-flight Windows launcher for scripts/e2e-docker.sh (Desktop Linux engine).

.DESCRIPTION
  Cap=0 fix for host exit 4294967295 (-1) observed when concurrent PowerShell
  wrappers fought over a shared console-*.log (OutFile IOException / Tee-Object
  lock) or when exclusive `*> log` redirection collided with a second launch.

  Rules:
  - Exactly one Desktop e2e flight at a time (mutex under .agent-runs/e2e/).
  - Unique host console log per RUN_ID — never reuse a fixed console-rN.log.
  - Prefer the authoritative bash runner's own tee (E2E_LOG_ROOT); host console
    is append-only via Start-Transcript, not exclusive Out-File redirection.
  - Force Docker Desktop affinity: PATH prepend + DOCKER_CONTEXT=desktop-linux.
  - Preserve the real bash/docker exit code (0-255). Never invent pass_product.

.PARAMETER SkipSwarm
  Sets E2E_SKIP_SWARM=1.

.PARAMETER SkipOverlays
  Sets E2E_SKIP_OVERLAYS=1 (default true on Desktop).

.PARAMETER Keep
  Sets E2E_KEEP=1.

.PARAMETER RunId
  Optional stable run id; default UTC stamp + project.

.PARAMETER AffinityOnly
  Cap=0 probe: verify affinity + mutex acquire/release; do not run e2e-docker.sh.
#>
param(
  [switch]$SkipSwarm,
  [switch]$SkipOverlays = $true,
  [switch]$Keep,
  [string]$RunId = "",
  [switch]$AffinityOnly
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

# --- Docker Desktop affinity (rule 96) --------------------------------------
$dockerBin = "C:\Program Files\Docker\Docker\resources\bin"
if (-not (Test-Path (Join-Path $dockerBin "docker.exe"))) {
  Write-Error "docker.exe missing under $dockerBin"
  exit 2
}
$env:PATH = "$dockerBin;" + $env:PATH
$env:DOCKER_CONTEXT = "desktop-linux"
$docker = Join-Path $dockerBin "docker.exe"

$engineId = (& $docker --context desktop-linux info --format "{{.ID}}").Trim()
$osType = (& $docker --context desktop-linux info --format "{{.OSType}}").Trim()
$endpoint = (& $docker --context desktop-linux context inspect desktop-linux --format "{{.Endpoints.docker.Host}}").Trim()
if ($osType -ne "linux" -or $endpoint -ne "npipe:////./pipe/dockerDesktopLinuxEngine") {
  Write-Error "Docker affinity failed: OSType=$osType endpoint=$endpoint (want linux + npipe dockerDesktopLinuxEngine)"
  exit 2
}
Write-Host "[e2e-launch] affinity ok engine=$engineId ostype=$osType endpoint=$endpoint"

# --- Single-flight mutex ----------------------------------------------------
$lockDir = Join-Path $Root ".agent-runs\e2e"
New-Item -ItemType Directory -Force -Path $lockDir | Out-Null
$lockPath = Join-Path $lockDir "single-flight.lock"
if (Test-Path $lockPath) {
  $holder = $null
  try { $holder = Get-Content -Raw -Path $lockPath | ConvertFrom-Json } catch { $holder = $null }
  if ($holder -and $holder.pid) {
    $alive = Get-Process -Id ([int]$holder.pid) -ErrorAction SilentlyContinue
    if ($alive) {
      $msg = "[e2e-launch] REFUSE: single-flight lock held by pid=$($holder.pid) runId=$($holder.runId) since=$($holder.since). Concurrent wrappers sharing one console log caused exit 4294967295 (-1)."
      # Use Host+exit 2 (not Write-Error under Stop) so callers see EXIT 2, never -1.
      Write-Host $msg -ForegroundColor Red
      exit 2
    }
  }
}

$project = if ($env:E2E_PROJECT_NAME) { $env:E2E_PROJECT_NAME } else { "hmn-e2e" }
$project = ($project.ToLowerInvariant() -replace '[^a-z0-9-]', '-').Trim('-')
if (-not $RunId) {
  $RunId = if ($env:E2E_RUN_ID) { $env:E2E_RUN_ID } else { "$(Get-Date -Format 'yyyyMMddTHHmmss')Z-$project" }
}
$env:E2E_PROJECT_NAME = $project
$env:E2E_RUN_ID = $RunId
if ($SkipSwarm) { $env:E2E_SKIP_SWARM = "1" }
if ($SkipOverlays) { $env:E2E_SKIP_OVERLAYS = "1" }
if ($Keep) { $env:E2E_KEEP = "1" }

$runDir = Join-Path $lockDir $RunId
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$hostConsole = Join-Path $runDir "host-console.log"
$env:E2E_LOG_ROOT = $lockDir

[ordered]@{
  pid = $PID
  runId = $RunId
  project = $project
  since = (Get-Date).ToUniversalTime().ToString("o")
  engineId = $engineId
  endpoint = $endpoint
} | ConvertTo-Json | Set-Content -Path $lockPath -Encoding utf8

if ($AffinityOnly) {
  Remove-Item -Force -Path $lockPath -ErrorAction SilentlyContinue
  Write-Host "[e2e-launch] AffinityOnly probe EXIT=0 runId=$RunId (no e2e-docker.sh; product unchanged=0)"
  exit 0
}

$bash = "C:\Program Files\Git\bin\bash.exe"
if (-not (Test-Path $bash)) {
  Remove-Item -Force -Path $lockPath -ErrorAction SilentlyContinue
  Write-Error "Git bash missing: $bash"
  exit 2
}

Write-Host "[e2e-launch] runId=$RunId project=$project hostConsole=$hostConsole"
Write-Host "[e2e-launch] invoking bash scripts/e2e-docker.sh (authoritative; unique E2E_RUN_ID)"

$exitCode = 1
try {
  # Transcript is append-safe; do NOT use exclusive `*> file` or Tee-Object onto a
  # shared console-rN.log (that produced OutFile IOException → exit -1 / 4294967295).
  Start-Transcript -Path $hostConsole -Append | Out-Null
  # Child inherits PATH + DOCKER_CONTEXT from this process.
  Push-Location $Root
  try {
    & $bash "./scripts/e2e-docker.sh"
    $exitCode = $LASTEXITCODE
  } finally {
    Pop-Location
  }
  if ($null -eq $exitCode) { $exitCode = 1 }
} finally {
  try { Stop-Transcript | Out-Null } catch {}
  Remove-Item -Force -Path $lockPath -ErrorAction SilentlyContinue
}

Write-Host "[e2e-launch] EXIT=$exitCode (preserved; product claim unchanged)"
exit $exitCode
