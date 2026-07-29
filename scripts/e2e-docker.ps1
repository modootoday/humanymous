#Requires -Version 5.1
<#
.SYNOPSIS
  Docker Desktop diagnostic subset (not completion authority).

.DESCRIPTION
  Convenience wrapper for local Docker Desktop. Linux CI and
  scripts/e2e-docker.sh remain completion authority. Product tests still run
  only in Linux containers; PowerShell only orchestrates Docker.

.PARAMETER SkipSwarm
  Skip multi-subnet swarm correlation.

.PARAMETER SkipOverlays
  Skip PLAN-08 compose overlay asserts. Defaults to true because the full
  overlay orchestrator is the Linux shell runner.

.PARAMETER Keep
  Leave the stack running after success.
#>
param(
  [switch]$SkipSwarm,
  [switch]$SkipOverlays = $true,
  [switch]$Keep
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

# Deterministic project name (NOT random). The lab networks pin FIXED subnets (networks.yaml,
# 172.30-32.0.0/24, internal:true) so the multi-subnet correlation test is real — but fixed subnets
# are SINGLETONS on the daemon, so only one stack can hold them at a time. A random per-run name made
# runs non-idempotent: a crashed run left an unfindable stack holding the subnets, and the next run
# collided ("Pool overlaps"). A stable name means a re-run replaces its own prior stack, and the
# pre-flight below frees the subnets from any OTHER holder — giving local the clean-slate guarantee
# CI gets for free on an ephemeral runner. Override with E2E_PROJECT_NAME.
$projectSeed = if ($env:E2E_PROJECT_NAME) { $env:E2E_PROJECT_NAME } else { "hmn-e2e" }
$project = ($projectSeed.ToLowerInvariant() -replace '[^a-z0-9-]', '-').Trim('-')
$composeArgs = @("-p", $project, "-f", "deployments/compose.yaml")
$runId = if ($env:E2E_RUN_ID) { $env:E2E_RUN_ID } else { "$(Get-Date -Format 'yyyyMMddTHHmmss')-$project" }
$runDir = Join-Path $Root ".agent-runs\e2e\$runId"
$logFile = Join-Path $runDir "e2e.log"
$statusFile = Join-Path $runDir "status.json"
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
Set-Content -Path (Join-Path $Root ".agent-runs\e2e\latest.log.path") -Value $logFile -Encoding utf8
Write-Host "[e2e-docker] live log: $logFile"
Write-Host "[e2e-docker] live status: $statusFile"
Start-Transcript -Path $logFile -Append | Out-Null

function Set-RunStatus {
  param([string]$Status, [string]$Phase, [int]$ExitCode = 0)
  [ordered]@{
    runId = $runId
    project = $project
    status = $Status
    phase = $Phase
    exitCode = $ExitCode
    updatedAt = (Get-Date).ToUniversalTime().ToString("o")
    log = $logFile
  } | ConvertTo-Json | Set-Content -Path $statusFile -Encoding utf8
}

function Invoke-DockerStep {
  param(
    [Parameter(Mandatory)][string]$Step,
    [Parameter(Mandatory)][string[]]$DockerArgs,
    [switch]$AllowFailure
  )
  $started = Get-Date
  Set-RunStatus -Status "running" -Phase $Step
  Write-Host ""
  Write-Host "========== [e2e-docker] START $Step @ $($started.ToString('o')) =========="
  Write-Host "[e2e-docker] COMMAND docker $($DockerArgs -join ' ')"
  & docker @DockerArgs 2>&1 | ForEach-Object { Write-Host $_ }
  $code = $LASTEXITCODE
  $elapsed = [int]((Get-Date) - $started).TotalSeconds
  if ($code -ne 0) {
    Set-RunStatus -Status "failed" -Phase $Step -ExitCode $code
    Write-Host "[e2e-docker] FAIL phase=$Step exit=$code elapsed=${elapsed}s" -ForegroundColor Red
    if (-not $AllowFailure) { throw "Docker step failed: $Step (exit $code)" }
  } else {
    Write-Host "========== [e2e-docker] PASS $Step elapsed=${elapsed}s =========="
  }
  return $code
}

function Invoke-Compose {
  param(
    [Parameter(Mandatory)][string]$Step,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Command
  )
  Invoke-DockerStep -Step $Step -DockerArgs (@("compose") + $composeArgs + $Command) | Out-Null
}

# Clear-LabSubnets frees the fixed lab subnets before we build/up, so a leftover stack (this
# project's own prior run, a `make up` under the default `deployments` project, or any other holder)
# can't block us with an opaque "Pool overlaps". This is the local equivalent of CI's ephemeral clean
# runner; on an already-clean daemon it is a no-op. Generic by design: it downs whatever compose
# project currently owns a `*_lab-[abc]` network, then removes any dangling lab network.
function Clear-LabSubnets {
  Write-Host "[e2e-docker] pre-flight: freeing lab subnets (removing any conflicting stack)..."
  & docker compose @composeArgs --profile swarm down -v --remove-orphans 2>$null | Out-Null
  $labNets = @(& docker network ls --format '{{.Name}}' 2>$null | Where-Object { $_ -match '_lab-[abc]$' })
  foreach ($n in $labNets) {
    $proj = (& docker network inspect $n --format '{{index .Labels "com.docker.compose.project"}}' 2>$null)
    if ($proj) {
      Write-Host "[e2e-docker] pre-flight: tearing down stack '$proj' holding $n"
      & docker compose -p $proj -f deployments/compose.yaml down -v --remove-orphans 2>$null | Out-Null
    }
    & docker network rm $n 2>$null | Out-Null
  }
  & docker network prune -f 2>$null | Out-Null
}

$cleanup = {
  if (-not $Keep) {
    Write-Host "[e2e-docker] tearing down stack..."
    & docker compose @composeArgs --profile swarm down -v --remove-orphans 2>$null | Out-Null
  } else {
    Write-Host "[e2e-docker] -Keep set — leaving stack running"
  }
}

try {
  Clear-LabSubnets
  Write-Host "[e2e-docker] validate compose..."
  # Use long Docker flags here: PowerShell can bind short flags such as -d to
  # the wrapper function's own common parameters instead of forwarding them.
  Invoke-Compose "validate-compose" config --quiet

  Invoke-Compose "build-images" --profile attack --profile gate-test --profile pass-test --profile swarm --progress plain build core gate bots
  Invoke-Compose "start-defenders" up --detach --no-build core origin gate

  Invoke-Compose "attack-catalog" run --rm --no-TTY bots

  Invoke-Compose "attack-assert" run --rm --no-TTY attack-assert

  Invoke-Compose "gate-conformance" run --rm --no-TTY gate-e2e

  Invoke-Compose "pass-contract" run --rm --no-TTY pass-e2e
  Invoke-Compose "restart-core" restart core
  Invoke-Compose "pass-wargame" run --rm --no-TTY pass-wargame

  if (-not $SkipSwarm) {
    Invoke-Compose "swarm-reset" run --rm --no-TTY swarm-reset
    Invoke-Compose "swarm" --profile swarm up --abort-on-container-failure bot-swarm-a bot-swarm-b bot-swarm-c
    Invoke-Compose "swarm-assert" run --rm --no-TTY swarm-assert
  } else {
    Write-Host "[e2e-docker] skip swarm"
  }

  if (-not $SkipOverlays) {
    throw "Full overlays require the authoritative Linux runner: bash scripts/e2e-docker.sh"
  }
  Write-Host "[e2e-docker] skip overlays (Docker Desktop diagnostics only)"

  Set-RunStatus -Status "completed" -Phase "complete"
  Write-Host "[e2e-docker] Docker Desktop diagnostic subset passed; this is not an E2E completion claim."
} finally {
  & $cleanup
  Stop-Transcript -ErrorAction SilentlyContinue | Out-Null
}

