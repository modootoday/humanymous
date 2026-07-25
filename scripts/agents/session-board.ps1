#Requires -Version 5.1
<#
.SYNOPSIS
  Multi-provider session board: list / claim / release work lanes.

.EXAMPLE
  pwsh -File scripts/agents/session-board.ps1 list
  pwsh -File scripts/agents/session-board.ps1 claim -Lane docs -Provider grok -Goal "fix docs" -Paths "docs/**"
  pwsh -File scripts/agents/session-board.ps1 release -Lane docs -Note "done"
#>
param(
  [Parameter(Position = 0)]
  [ValidateSet("list", "claim", "release", "init")]
  [string]$Action = "list",

  [string]$Lane = "",
  [ValidateSet("claude", "grok", "codex", "gemini", "human", "")]
  [string]$Provider = "",
  [string]$Goal = "",
  [string]$Paths = "",
  [string]$Session = "",
  [string]$Note = "",
  [switch]$Force
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$BoardDir = Join-Path $Root ".agents\sessions"
$Active = Join-Path $BoardDir "ACTIVE.md"
$Example = Join-Path $BoardDir "ACTIVE.example.md"
$ClaimsDir = Join-Path $BoardDir "claims"
$LanesDoc = Join-Path $BoardDir "LANES.md"
$TtlHours = 4
$GitOpsTtlMinutes = 30

$ValidLanes = @(
  "detection-core", "gate-edge", "pass", "red-catalog", "e2e-infra",
  "docs", "agents-meta", "sot-plan", "release", "git-ops", "misc"
)

function Ensure-Board {
  if (-not (Test-Path $ClaimsDir)) {
    New-Item -ItemType Directory -Path $ClaimsDir -Force | Out-Null
  }
  if (-not (Test-Path $Active)) {
    if (Test-Path $Example) {
      Copy-Item $Example $Active
      Write-Host "Seeded ACTIVE.md from ACTIVE.example.md"
    } else {
      @"
# ACTIVE session board

Last updated: $(Get-Date -Format "yyyy-MM-dd HH:mm")

## Claims

| Lane | Provider | Session | Goal | Paths | Since | Status |
|------|----------|---------|------|-------|-------|--------|
| — | — | — | — | — | — | free |

## Handovers (incomplete work)

"@ | Set-Content -Path $Active -Encoding utf8
    }
  }
}

function Get-ClaimPath([string]$L) {
  Join-Path $ClaimsDir "$L.json"
}

function Read-Claim([string]$L) {
  $p = Get-ClaimPath $L
  if (-not (Test-Path $p)) { return $null }
  return (Get-Content $p -Raw | ConvertFrom-Json)
}

function Is-Stale($claim) {
  if (-not $claim) { return $false }
  try {
    $since = [datetime]::Parse([string]$claim.since)
    $age = (Get-Date) - $since
    # git-ops is short-lived; long holds are almost always abandoned mid-transaction
    if ($claim.lane -eq "git-ops") {
      return $age.TotalMinutes -gt $GitOpsTtlMinutes
    }
    return $age.TotalHours -gt $TtlHours
  } catch {
    return $true
  }
}

function Write-ActiveMarkdown {
  $rows = @()
  foreach ($L in $ValidLanes) {
    $c = Read-Claim $L
    if (-not $c) { continue }
    $status = if (Is-Stale $c) { "stale" } else { $c.status }
    $rows += "| $($c.lane) | $($c.provider) | $($c.session) | $($c.goal) | $($c.paths) | $($c.since) | $status |"
  }
  if ($rows.Count -eq 0) {
    $rows = @("| — | — | — | — | — | — | free |")
  }
  $handover = ""
  if (Test-Path $Active) {
    $prev = Get-Content $Active -Raw
    if ($prev -match '(?s)## Handovers \(incomplete work\)\s*(.*)$') {
      $handover = $Matches[1].Trim()
    }
  }
  if (-not $handover) { $handover = "_None._" }

  @"
# ACTIVE session board

Last updated: $(Get-Date -Format "yyyy-MM-dd HH:mm")

## Claims

| Lane | Provider | Session | Goal | Paths | Since | Status |
|------|----------|---------|------|-------|-------|--------|
$($rows -join "`n")

## Handovers (incomplete work)

$handover
"@ | Set-Content -Path $Active -Encoding utf8
}

function Action-List {
  Ensure-Board
  Write-Host "Lanes catalog: $LanesDoc"
  Write-Host "Board: $Active"
  Write-Host ""
  $any = $false
  foreach ($L in $ValidLanes) {
    $c = Read-Claim $L
    if (-not $c) { continue }
    $any = $true
    $stale = Is-Stale $c
    $mark = if ($stale) { "STALE" } else { $c.status }
    Write-Host ("[{0}] {1} provider={2} session={3}" -f $mark, $c.lane, $c.provider, $c.session)
    Write-Host ("       goal: {0}" -f $c.goal)
    Write-Host ("       paths: {0}" -f $c.paths)
    Write-Host ("       since: {0}" -f $c.since)
  }
  if (-not $any) {
    Write-Host "(no active claims — all lanes free)"
  }
  Write-ActiveMarkdown | Out-Null
}

function Action-Claim {
  Ensure-Board
  if ($ValidLanes -notcontains $Lane) {
    throw "Unknown lane '$Lane'. Valid: $($ValidLanes -join ', ')"
  }
  if (-not $Provider) { throw "-Provider is required (claude|grok|codex|gemini|human)" }
  if (-not $Goal) { throw "-Goal is required" }
  if (-not $Paths) { $Paths = "(see LANES.md for $Lane)" }
  if (-not $Session) { $Session = "local-$(Get-Date -Format 'HHmmss')" }

  $existing = Read-Claim $Lane
  if ($existing -and -not $Force) {
    if (-not (Is-Stale $existing)) {
      throw "Lane '$Lane' is held by $($existing.provider)/$($existing.session) since $($existing.since). Use -Force only if abandoned."
    }
    Write-Host "Replacing STALE claim on $Lane"
  }

  # Soft mutex warning
  if ($Lane -eq "detection-core") {
    $g = Read-Claim "gate-edge"
    if ($g -and -not (Is-Stale $g)) {
      Write-Host "WARNING: gate-edge is also claimed — scoring seams may conflict." -ForegroundColor Yellow
    }
  }
  if ($Lane -eq "gate-edge") {
    $d = Read-Claim "detection-core"
    if ($d -and -not (Is-Stale $d)) {
      Write-Host "WARNING: detection-core is also claimed — scoring seams may conflict." -ForegroundColor Yellow
    }
  }
  if ($Lane -eq "git-ops") {
    $lock = Join-Path $Root ".git\index.lock"
    if (Test-Path $lock) {
      Write-Host "WARNING: .git/index.lock present — resolve before git writes." -ForegroundColor Yellow
    }
    Write-Host "NOTE: release git-ops immediately after commit/push (TTL ${GitOpsTtlMinutes}m)." -ForegroundColor Cyan
  }
  if ($Lane -eq "release") {
    $go = Read-Claim "git-ops"
    if ($go -and -not (Is-Stale $go)) {
      Write-Host "WARNING: git-ops already held — serialize release tagging with git-ops holder." -ForegroundColor Yellow
    }
  }

  $claim = [ordered]@{
    lane     = $Lane
    provider = $Provider
    session  = $Session
    goal     = $Goal
    paths    = $Paths
    since    = (Get-Date -Format "o")
    status   = "active"
  }
  ($claim | ConvertTo-Json) | Set-Content -Path (Get-ClaimPath $Lane) -Encoding utf8
  Write-ActiveMarkdown
  Write-Host "Claimed lane=$Lane provider=$Provider session=$Session"
}

function Action-Release {
  Ensure-Board
  if (-not $Lane) { throw "-Lane is required" }
  $existing = Read-Claim $Lane
  if (-not $existing -and -not $Force) {
    Write-Host "Lane $Lane already free"
    return
  }
  if ($existing -and (Is-Stale $existing) -eq $false -and $Note -eq "" -and -not $Force) {
    # ok to release own work without note
  }
  $p = Get-ClaimPath $Lane
  if (Test-Path $p) { Remove-Item $p -Force }

  if ($Note) {
    $block = @"

### $(Get-Date -Format "yyyy-MM-dd HH:mm") — release $Lane

- **Note:** $Note
- **Was:** $(if ($existing) { "$($existing.provider)/$($existing.session): $($existing.goal)" } else { "n/a" })
"@
    if (-not (Test-Path $Active)) { Ensure-Board }
    Add-Content -Path $Active -Value $block -Encoding utf8
  }
  Write-ActiveMarkdown
  Write-Host "Released lane=$Lane"
}

switch ($Action) {
  "init" { Ensure-Board; Write-Host "Board ready at $Active" }
  "list" { Action-List }
  "claim" { Action-Claim }
  "release" { Action-Release }
}
