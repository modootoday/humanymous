#Requires -Version 5.1
<#
.SYNOPSIS
  Fail if recent commits lack project Co-Authored-By agent trailers.

.EXAMPLE
  pwsh -File scripts/agents/check-commit-attribution.ps1
  pwsh -File scripts/agents/check-commit-attribution.ps1 -Since HEAD~30 -RequireAll
#>
param(
  [string]$Since = "HEAD~30",
  [switch]$RequireAll
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

# Substrings that must appear for a known agent trailer (GitHub-linked emails).
# See .agents/sessions/COMMIT-CONVENTION.md
$KnownEmailOrId = @(
  "noreply@anthropic.com",
  "claude@anthropic.com",  # legacy Claude variant
  "codex@openai.com",
  "noreply@openai.com",
  "chatgpt-codex-connector[bot]@users.noreply.github.com",
  "grok@x.ai",
  "noreply@x.ai",
  "gemini-cli@users.noreply.github.com",
  # legacy (pre community-email / pre project-local canons) — still accepted
  "304785771+grokkybara[bot]@users.noreply.github.com",
  "grokkybara[bot]@users.noreply.github.com",
  "claude-code@agents.humanymous.local",
  "grok-build@agents.humanymous.local",
  "openai-codex@agents.humanymous.local",
  "gemini-cli@agents.humanymous.local"
)

$range = git rev-list "$Since..HEAD" 2>$null
if (-not $range) {
  $range = git rev-list -n 30 HEAD
}

$failed = 0
$checked = 0
foreach ($sha in ($range -split "\r?\n" | Where-Object { $_ })) {
  $checked++
  $body = git log -1 --format=%B $sha
  $subject = git log -1 --format=%s $sha
  $has = $false

  if ($body -match '(?im)^Co-Authored-By:\s*.+@.+\S') {
    foreach ($k in $KnownEmailOrId) {
      if ($body -like "*$k*") {
        $has = $true
        break
      }
    }
    # Any Co-Authored-By with email: accept unless -RequireAll demands registry
    if (-not $has -and -not $RequireAll) {
      $has = $true
    }
  }

  if (-not $has) {
    if ($RequireAll -or $subject -match '^(feat|fix|docs|test|ci|build|refactor|perf|harden|security|chore)(\(|:|!)') {
      Write-Host "FAIL $sha  missing agent Co-Authored-By  $subject" -ForegroundColor Red
      $failed++
    } else {
      Write-Host "SKIP $sha  $subject"
    }
  } else {
    Write-Host "OK   $sha  $subject"
  }
}

Write-Host ""
Write-Host "checked=$checked failed=$failed"
if ($failed -gt 0) { exit 1 }
exit 0
