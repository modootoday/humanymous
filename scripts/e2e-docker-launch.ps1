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

.PARAMETER PressureOnly
  Cap=0 probe: affinity + Desktop VM memory/pressure guard only; do not run e2e-docker.sh.
#>
param(
  [switch]$SkipSwarm,
  [switch]$SkipOverlays = $true,
  [switch]$Keep,
  [string]$RunId = "",
  [switch]$AffinityOnly,
  [switch]$PressureOnly
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

# --- Cap=0 Desktop VM memory/pressure guard (fail-closed) -------------------
# Full e2e attempt2 EXIT137: bots SIGKILL mid-catalog after patchright while
# host FreeGB~27 remained — Desktop Linux VM pressure, not host RAM. Prefer
# refuse EXIT 2 before the burn over inventing pass_product after OOM.
function Get-KbFromMeminfoLine {
  param([string[]]$Lines, [string]$Key)
  foreach ($line in $Lines) {
    if ($line -match ("^" + [regex]::Escape($Key) + ":\s+(\d+)\s+kB")) {
      return [int64]$Matches[1]
    }
  }
  return $null
}

function Invoke-DesktopPressureGuard {
  param(
    [Parameter(Mandatory)][string]$DockerExe,
    [Parameter(Mandatory)][string]$ReceiptPath,
    [bool]$SwarmEnabled
  )
  $guardOn = if ($env:E2E_PRESSURE_GUARD) { $env:E2E_PRESSURE_GUARD } else { "1" }
  if ($guardOn -eq "0") {
    Write-Host "[e2e-launch] pressure guard skipped (E2E_PRESSURE_GUARD=0)"
    return $true
  }

  $minAvail = if ($env:E2E_MIN_VM_MEM_AVAILABLE_MB) { [int]$env:E2E_MIN_VM_MEM_AVAILABLE_MB } else { if ($SwarmEnabled) { 16384 } else { 12288 } }
  $minSwap = if ($env:E2E_MIN_VM_SWAP_FREE_MB) { [int]$env:E2E_MIN_VM_SWAP_FREE_MB } else { 512 }
  $minTotal = if ($env:E2E_MIN_VM_MEM_TOTAL_MB) { [int]$env:E2E_MIN_VM_MEM_TOTAL_MB } else { 16384 }
  $minHost = if ($env:E2E_MIN_HOST_FREE_MB) { [int]$env:E2E_MIN_HOST_FREE_MB } else { 4096 }

  $os = Get-CimInstance Win32_OperatingSystem
  $hostFreeMb = [int][math]::Floor($os.FreePhysicalMemory / 1024.0)
  $hostTotalMb = [int][math]::Floor($os.TotalVisibleMemorySize / 1024.0)

  $probeImages = @("humanymous/bots:local", "busybox:1.36", "busybox:latest", "alpine:3.20")
  $meminfoText = $null
  $probeImage = $null
  foreach ($img in $probeImages) {
    & $DockerExe --context desktop-linux image inspect $img *> $null
    if ($LASTEXITCODE -ne 0) { continue }
    $meminfoText = & $DockerExe --context desktop-linux run --rm --network none --entrypoint cat $img /proc/meminfo 2>$null
    if ($LASTEXITCODE -eq 0 -and $meminfoText) {
      $probeImage = $img
      break
    }
  }

  $reasons = New-Object System.Collections.Generic.List[string]
  if (-not $meminfoText) {
    $reasons.Add("desktop_vm_meminfo_unreadable")
  }

  $lines = @()
  if ($meminfoText) {
    if ($meminfoText -is [array]) { $lines = $meminfoText } else { $lines = ($meminfoText -split "`r?`n") }
  }
  $memTotalKb = Get-KbFromMeminfoLine -Lines $lines -Key "MemTotal"
  $memAvailKb = Get-KbFromMeminfoLine -Lines $lines -Key "MemAvailable"
  $swapFreeKb = Get-KbFromMeminfoLine -Lines $lines -Key "SwapFree"
  $swapTotalKb = Get-KbFromMeminfoLine -Lines $lines -Key "SwapTotal"

  $vmTotalMb = if ($null -ne $memTotalKb) { [int][math]::Floor($memTotalKb / 1024.0) } else { $null }
  $vmAvailMb = if ($null -ne $memAvailKb) { [int][math]::Floor($memAvailKb / 1024.0) } else { $null }
  $vmSwapFreeMb = if ($null -ne $swapFreeKb) { [int][math]::Floor($swapFreeKb / 1024.0) } else { $null }
  $vmSwapTotalMb = if ($null -ne $swapTotalKb) { [int][math]::Floor($swapTotalKb / 1024.0) } else { $null }

  if ($null -eq $vmAvailMb) { $reasons.Add("MemAvailable_missing") }
  elseif ($vmAvailMb -lt $minAvail) { $reasons.Add("MemAvailable_MB=$vmAvailMb<min=$minAvail") }

  if ($null -eq $vmTotalMb) { $reasons.Add("MemTotal_missing") }
  elseif ($vmTotalMb -lt $minTotal) { $reasons.Add("MemTotal_MB=$vmTotalMb<min=$minTotal") }

  if ($null -eq $vmSwapFreeMb) { $reasons.Add("SwapFree_missing") }
  elseif ($vmSwapFreeMb -lt $minSwap) { $reasons.Add("SwapFree_MB=$vmSwapFreeMb<min=$minSwap") }

  if ($hostFreeMb -lt $minHost) { $reasons.Add("hostFree_MB=$hostFreeMb<min=$minHost") }

  $ok = ($reasons.Count -eq 0)
  $receipt = [ordered]@{
    schema = "humanymous.e2e-desktop-pressure-guard/1"
    observed_utc = (Get-Date).ToUniversalTime().ToString("o")
    result = $(if ($ok) { "PASS" } else { "REFUSE" })
    pass_product_clean_room = 0
    product = 0
    swarm_enabled = $SwarmEnabled
    probe_image = $probeImage
    thresholds_mb = [ordered]@{
      min_vm_mem_available = $minAvail
      min_vm_swap_free = $minSwap
      min_vm_mem_total = $minTotal
      min_host_free = $minHost
    }
    observed_mb = [ordered]@{
      vm_mem_total = $vmTotalMb
      vm_mem_available = $vmAvailMb
      vm_swap_total = $vmSwapTotalMb
      vm_swap_free = $vmSwapFreeMb
      host_mem_total = $hostTotalMb
      host_mem_free = $hostFreeMb
    }
    refuse_reasons = @($reasons)
    note = "Fail-closed Cap=0 guard against Desktop VM EXIT137; never invents pass_product_clean_room"
  }
  ($receipt | ConvertTo-Json -Depth 6) | Set-Content -Path $ReceiptPath -Encoding utf8

  if (-not $ok) {
    Write-Host "[e2e-launch] REFUSE pressure-guard: $($reasons -join '; ') (receipt=$ReceiptPath)" -ForegroundColor Red
    Write-Host "[e2e-launch] Raise Docker Desktop VM memory/swap or free Desktop pressure, then retry once. product=0." -ForegroundColor Red
    return $false
  }
  Write-Host "[e2e-launch] pressure-guard PASS vmAvailMB=$vmAvailMb swapFreeMB=$vmSwapFreeMb hostFreeMB=$hostFreeMb (receipt=$ReceiptPath)"
  return $true
}

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

# Cap=0 bots containment defaults (compose run flags; do not invent product pass).
if (-not $env:E2E_BOTS_SHM_SIZE) { $env:E2E_BOTS_SHM_SIZE = "1g" }
if (-not $env:E2E_BOTS_MEMORY) { $env:E2E_BOTS_MEMORY = "12g" }
# Force serial catalog intensity on Desktop (runner is already serial; jobs=1 for any build).
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = "1" }

$runDir = Join-Path $lockDir $RunId
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$hostConsole = Join-Path $runDir "host-console.log"
$env:E2E_LOG_ROOT = $lockDir
$pressureReceipt = Join-Path $runDir "pressure-guard.json"

[ordered]@{
  pid = $PID
  runId = $RunId
  project = $project
  since = (Get-Date).ToUniversalTime().ToString("o")
  engineId = $engineId
  endpoint = $endpoint
} | ConvertTo-Json | Set-Content -Path $lockPath -Encoding utf8

$swarmEnabled = -not $SkipSwarm
if (-not (Invoke-DesktopPressureGuard -DockerExe $docker -ReceiptPath $pressureReceipt -SwarmEnabled:$swarmEnabled)) {
  Remove-Item -Force -Path $lockPath -ErrorAction SilentlyContinue
  exit 2
}

if ($AffinityOnly -or $PressureOnly) {
  Remove-Item -Force -Path $lockPath -ErrorAction SilentlyContinue
  $mode = if ($PressureOnly) { "PressureOnly" } else { "AffinityOnly" }
  Write-Host "[e2e-launch] $mode probe EXIT=0 runId=$RunId (no e2e-docker.sh; product unchanged=0)"
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
