#Requires -Version 5.1
<#
.SYNOPSIS
  Authoritative Docker-only e2e suite (Windows-friendly).

.DESCRIPTION
  Mirrors scripts/e2e-docker.sh. All e2e must use the compose stack — host/loopback
  is not completion authority for network-plane detection.

.PARAMETER SkipSwarm
  Skip multi-subnet swarm correlation.

.PARAMETER SkipOverlays
  Skip PLAN-08 compose overlay asserts.

.PARAMETER Keep
  Leave the stack running after success.
#>
param(
  [switch]$SkipSwarm,
  [switch]$SkipOverlays,
  [switch]$Keep
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

$projectSeed = if ($env:E2E_PROJECT_NAME) { $env:E2E_PROJECT_NAME } else { "hmn-e2e-$([guid]::NewGuid().ToString('N').Substring(0, 8))" }
$project = ($projectSeed.ToLowerInvariant() -replace '[^a-z0-9-]', '-').Trim('-')
$composeArgs = @("-p", $project, "-f", "deployments/compose.yaml")
function Invoke-Compose {
  param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Args)
  & docker compose @composeArgs @Args
  if ($LASTEXITCODE -ne 0) { throw "docker compose failed: $Args (exit $LASTEXITCODE)" }
}

$cleanup = {
  if (-not $Keep) {
    Write-Host "[e2e-docker] tearing down stack..."
    & docker compose @composeArgs down -v 2>$null | Out-Null
  } else {
    Write-Host "[e2e-docker] -Keep set — leaving stack running"
  }
}

try {
  Write-Host "[e2e-docker] validate compose..."
  Invoke-Compose config -q

  Write-Host "[e2e-docker] build + start defenders..."
  Invoke-Compose up -d --build core origin gate

  Write-Host "[e2e-docker] automation catalog (bots)..."
  Invoke-Compose run --rm bots

  Write-Host "[e2e-docker] assert attack gate..."
  Invoke-Compose run --rm attack-assert

  Write-Host "[e2e-docker] gate proxy-layer conformance..."
  Invoke-Compose run --rm gate-e2e

  Write-Host "[e2e-docker] Pass contract..."
  Invoke-Compose run --rm pass-e2e
  Invoke-Compose restart core
  Write-Host "[e2e-docker] Pass wargame..."
  Invoke-Compose run --rm pass-wargame

  if (-not $SkipSwarm) {
    Write-Host "[e2e-docker] multi-subnet swarm..."
    Invoke-Compose run --rm swarm-reset
    Invoke-Compose --profile swarm up --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c
    Invoke-Compose run --rm swarm-assert
    Write-Host "[e2e-docker] swarm correlation OK"
  } else {
    Write-Host "[e2e-docker] skip swarm"
  }

  if (-not $SkipOverlays) {
    if (Test-Path "scripts/ci-overlays.sh") {
      Write-Host "[e2e-docker] PLAN-08 overlays..."
      Invoke-Compose down -v
      $env:E2E_PROJECT_NAME = "$project-overlay"
      & docker run --rm `
        -v /var/run/docker.sock:/var/run/docker.sock `
        -v "${Root}:/workspace" `
        -w /workspace `
        -e "E2E_PROJECT_NAME=$env:E2E_PROJECT_NAME" `
        docker:27-cli sh scripts/ci-overlays.sh
      if ($LASTEXITCODE -ne 0) { throw "Docker overlay orchestrator failed" }
    }
  } else {
    Write-Host "[e2e-docker] skip overlays"
  }

  Write-Host "[e2e-docker] ALL e2e gates passed."
} finally {
  & $cleanup
}

