#Requires -Version 5.1
<#
.SYNOPSIS
  Coordinate exclusive git write operations across multi-provider sessions.

.EXAMPLE
  pwsh -File scripts/agents/git-coord.ps1 preflight
  pwsh -File scripts/agents/git-coord.ps1 claim -Provider grok -Session "abc"
  pwsh -File scripts/agents/git-coord.ps1 release -Note "pushed"
#>
param(
  [Parameter(Position = 0)]
  [ValidateSet("preflight", "claim", "release", "status")]
  [string]$Action = "preflight",

  [ValidateSet("claude", "grok", "codex", "gemini", "human", "")]
  [string]$Provider = "human",
  [string]$Session = "",
  [string]$Note = "",
  [string]$Goal = "git write transaction",
  [switch]$Force
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root
$Board = Join-Path $Root "scripts\agents\session-board.ps1"
$ClaimsDir = Join-Path $Root ".agents\sessions\claims"
$GitOpsClaim = Join-Path $ClaimsDir "git-ops.json"
$IndexLock = Join-Path $Root ".git\index.lock"
$GitOpsTtlMinutes = 30

function Invoke-Board {
  param([string[]]$BoardArgs)
  & pwsh -NoProfile -File $Board @BoardArgs
  if ($LASTEXITCODE -ne 0) { throw "session-board failed: $($BoardArgs -join ' ')" }
}

function Get-GitOpsClaim {
  if (-not (Test-Path $GitOpsClaim)) { return $null }
  return (Get-Content $GitOpsClaim -Raw | ConvertFrom-Json)
}

function Test-GitOpsStale($claim) {
  if (-not $claim) { return $false }
  try {
    $since = [datetime]::Parse([string]$claim.since)
    return ((Get-Date) - $since).TotalMinutes -gt $GitOpsTtlMinutes
  } catch { return $true }
}

function Action-Preflight {
  Write-Host "=== git-coord preflight ==="
  if (-not (Test-Path (Join-Path $Root ".git"))) {
    throw "Not a git repository: $Root"
  }

  if (Test-Path $IndexLock) {
    throw "BLOCKED: .git/index.lock exists — another git process may be running. Wait or diagnose before writing."
  }
  Write-Host "OK  no index.lock"

  $c = Get-GitOpsClaim
  if ($c) {
    $stale = Test-GitOpsStale $c
    if (-not $stale) {
      throw "BLOCKED: git-ops held by $($c.provider)/$($c.session) since $($c.since) goal=$($c.goal)"
    }
    Write-Host "WARN git-ops claim STALE (>$GitOpsTtlMinutes min) — release -Force before reclaim" -ForegroundColor Yellow
  } else {
    Write-Host "OK  git-ops free"
  }

  # git status summary
  $sb = git status -sb 2>&1 | Out-String
  Write-Host "--- git status -sb ---"
  Write-Host $sb.TrimEnd()

  # warn if many uncommitted paths and multiple work claims
  if (Test-Path $ClaimsDir) {
    $others = Get-ChildItem $ClaimsDir -Filter "*.json" | Where-Object { $_.BaseName -ne "git-ops" }
    if ($others.Count -gt 1) {
      Write-Host "WARN multiple work-lane claims active ($($others.Count)) — stage only your paths" -ForegroundColor Yellow
      foreach ($o in $others) {
        $j = Get-Content $o.FullName -Raw | ConvertFrom-Json
        Write-Host "  lane=$($j.lane) provider=$($j.provider) paths=$($j.paths)"
      }
    }
  }

  # upstream divergence
  git rev-parse --abbrev-ref '@{u}' 2>$null | Out-Null
  if ($LASTEXITCODE -eq 0) {
    git fetch --quiet 2>$null
    $counts = git rev-list --left-right --count 'HEAD...@{u}' 2>$null
    if ($counts) {
      $parts = ($counts -split '\s+')
      if ($parts.Count -ge 2) {
        Write-Host "OK  ahead $($parts[0]) / behind $($parts[1]) vs upstream"
        if ([int]$parts[1] -gt 0) {
          Write-Host "WARN branch is BEHIND upstream — pull/rebase under git-ops before push" -ForegroundColor Yellow
        }
      }
    }
  } else {
    Write-Host "NOTE no upstream configured for current branch"
  }

  Write-Host "=== preflight done (claim git-ops before add/commit/push) ==="
}

function Action-Claim {
  if (-not $Session) { $Session = "git-$(Get-Date -Format 'HHmmss')" }
  $boardArgs = @(
    "claim",
    "-Lane", "git-ops",
    "-Provider", $Provider,
    "-Goal", $Goal,
    "-Paths", ".git/** (index/HEAD/ref writes only)",
    "-Session", $Session
  )
  if ($Force) { $boardArgs += "-Force" }
  Invoke-Board -BoardArgs $boardArgs
  Write-Host "Hold git-ops only for the transaction; release immediately after."
}

function Action-Release {
  $boardArgs = @("release", "-Lane", "git-ops")
  if ($Note) { $boardArgs += @("-Note", $Note) }
  if ($Force) { $boardArgs += "-Force" }
  Invoke-Board -BoardArgs $boardArgs
}

function Action-Status {
  $c = Get-GitOpsClaim
  if (-not $c) {
    Write-Host "git-ops: free"
  } else {
    $stale = Test-GitOpsStale $c
    Write-Host ("git-ops: {0} provider={1} session={2} since={3} goal={4}" -f `
      ($(if ($stale) { "STALE" } else { $c.status })), $c.provider, $c.session, $c.since, $c.goal)
  }
  if (Test-Path $IndexLock) {
    Write-Host "index.lock: PRESENT" -ForegroundColor Red
  } else {
    Write-Host "index.lock: absent"
  }
  git status -sb
}

switch ($Action) {
  "preflight" { Action-Preflight }
  "claim" { Action-Claim }
  "release" { Action-Release }
  "status" { Action-Status }
}
