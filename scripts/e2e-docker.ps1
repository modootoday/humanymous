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

$projectSeed = if ($env:E2E_PROJECT_NAME) { $env:E2E_PROJECT_NAME } else { "hmn-e2e-$([guid]::NewGuid().ToString('N').Substring(0, 8))" }
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

$cleanup = {
  if (-not $Keep) {
    Write-Host "[e2e-docker] tearing down stack..."
    & docker compose @composeArgs --profile swarm down -v 2>$null | Out-Null
  } else {
    Write-Host "[e2e-docker] -Keep set — leaving stack running"
  }
}

try {
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

