#Requires -Version 5.1
<#
.SYNOPSIS
  Coordinate exclusive git write operations across multi-provider sessions.

.EXAMPLE
  pwsh -File scripts/agents/git-coord.ps1 preflight
  pwsh -File scripts/agents/git-coord.ps1 claim -Provider grok -Session "abc"
  pwsh -File scripts/agents/git-coord.ps1 commit -Provider grok -Subject "feat(agents): …" -Body "…"
  pwsh -File scripts/agents/git-coord.ps1 release -Note "pushed"
#>
param(
  [Parameter(Position = 0)]
  [ValidateSet("preflight", "claim", "release", "status", "commit")]
  [string]$Action = "preflight",

  [ValidateSet("claude", "grok", "codex", "gemini", "human", "")]
  [string]$Provider = "human",
  [string]$Session = "",
  [string]$Note = "",
  [string]$Goal = "git write transaction",
  [string]$Subject = "",
  [string]$Body = "",
  [string]$MessageFile = "",
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

  Write-Host "NOTE agent commits MUST use: git-coord commit -Provider <claude|grok|codex|gemini> -Subject 'type: …'"
  Write-Host "     (injects Co-Authored-By per .agents/sessions/COMMIT-CONVENTION.md)"
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

function Get-ProviderIdentity([string]$P) {
  # Emails must be GitHub-linked for avatars — see .agents/sessions/COMMIT-CONVENTION.md
  switch ($P) {
    "claude" { return @{ Display = "Claude"; Email = "noreply@anthropic.com" } }
    "grok"   { return @{ Display = "Grok"; Email = "grok@x.ai" } }
    "codex"  { return @{ Display = "codex"; Email = "codex@openai.com" } }
    "gemini" { return @{ Display = "Gemini CLI"; Email = "gemini-cli@users.noreply.github.com" } }
    default { throw "commit requires -Provider claude|grok|codex|gemini (not '$P')" }
  }
}

function Action-Commit {
  $c = Get-GitOpsClaim
  if (-not $c -or (Test-GitOpsStale $c)) {
    throw "git-ops must be actively claimed before commit (git-coord claim -Provider …)"
  }
  if ($Provider -and $c.provider -ne $Provider -and -not $Force) {
    Write-Host "WARN claim provider=$($c.provider) but -Provider $Provider (use -Force to override)" -ForegroundColor Yellow
  }
  $useProvider = if ($Provider -and $Provider -ne "human" -and $Provider -ne "") { $Provider } else { $c.provider }
  $id = Get-ProviderIdentity $useProvider

  $subjectText = $Subject
  $bodyText = $Body
  if ($MessageFile) {
    if (-not (Test-Path $MessageFile)) { throw "MessageFile not found: $MessageFile" }
    $lines = Get-Content $MessageFile
    if ($lines.Count -lt 1) { throw "MessageFile empty" }
    $subjectText = $lines[0]
    if ($lines.Count -gt 2) {
      $bodyText = ($lines[2..($lines.Count - 1)] -join "`n").Trim()
    } elseif ($lines.Count -eq 2 -and $lines[1].Trim() -ne "") {
      $bodyText = $lines[1].Trim()
    }
  }
  if (-not $subjectText) { throw "-Subject is required (or -MessageFile)" }
  if ($subjectText -notmatch '^(feat|fix|docs|test|ci|build|refactor|perf|harden|security|chore)(\(.+\))?!?: ') {
    throw "Subject must be Conventional Commits, e.g. feat(scope): summary"
  }

  $trailer = "Co-Authored-By: $($id.Display) <$($id.Email)>"
  $xprov = "X-Agent-Provider: $useProvider"
  $msg = $subjectText.TrimEnd() + "`n"
  if ($bodyText -and $bodyText.Trim().Length -gt 0) {
    $msg += "`n" + $bodyText.Trim() + "`n"
  }
  $msg += "`n" + $trailer + "`n" + $xprov + "`n"

  $tmp = Join-Path $env:TEMP ("hmn-commit-" + [guid]::NewGuid().ToString("N") + ".txt")
  Set-Content -Path $tmp -Value $msg -Encoding utf8
  try {
    git commit -F $tmp
    if ($LASTEXITCODE -ne 0) { throw "git commit failed ($LASTEXITCODE)" }
    $sha = git rev-parse --short HEAD
    Write-Host "OK committed $sha with $trailer"
  } finally {
    Remove-Item $tmp -Force -ErrorAction SilentlyContinue
  }
}

switch ($Action) {
  "preflight" { Action-Preflight }
  "claim" { Action-Claim }
  "release" { Action-Release }
  "status" { Action-Status }
  "commit" { Action-Commit }
}
