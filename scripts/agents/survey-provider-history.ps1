#Requires -Version 5.1
<#
.SYNOPSIS
  Inventory Claude / Codex / Gemini / Grok session artifacts for this project.
  Does not print transcript bodies or secrets.
#>
$ErrorActionPreference = "Continue"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$marker = "automation-blocking-skills"

function Section([string]$Title) {
  Write-Host ""
  Write-Host "=== $Title ===" -ForegroundColor Cyan
}

Section "Workspace"
Write-Host $Root

Section "Claude Code"
$claudeProj = Get-ChildItem "$env:USERPROFILE\.claude\projects" -Directory -ErrorAction SilentlyContinue |
  Where-Object { $_.Name -match "automation-blocking" }
if ($claudeProj) {
  foreach ($p in $claudeProj) {
    Write-Host "project: $($p.FullName)"
    $sessions = Get-ChildItem $p -Filter "*.jsonl" -ErrorAction SilentlyContinue
    Write-Host "  session jsonl: $($sessions.Count)"
    $mem = Join-Path $p.FullName "memory"
    if (Test-Path $mem) {
      $n = (Get-ChildItem $mem -Filter "*.md").Count
      Write-Host "  memory md files: $n"
      $idx = Join-Path $mem "MEMORY.md"
      if (Test-Path $idx) {
        Write-Host "  MEMORY.md: $((Get-Item $idx).Length) bytes, $((Get-Item $idx).LastWriteTime)"
      }
    }
    Get-ChildItem $p -Directory | Where-Object { $_.Name -match '^[0-9a-f-]{36}$' } |
      Sort-Object LastWriteTime -Descending | Select-Object -First 5 |
      ForEach-Object { Write-Host "  session dir: $($_.Name)  $($_.LastWriteTime)" }
  }
} else {
  Write-Host "(no Claude project dir for this repo)"
}

Section "OpenAI Codex"
$codexHits = @()
if (Test-Path "$env:USERPROFILE\.codex\sessions") {
  Get-ChildItem "$env:USERPROFILE\.codex\sessions" -Recurse -Filter "*.jsonl" -ErrorAction SilentlyContinue |
    ForEach-Object {
      if (Select-String -Path $_.FullName -Pattern $marker -SimpleMatch -Quiet -ErrorAction SilentlyContinue) {
        $codexHits += $_
      }
    }
}
Write-Host "jsonl mentioning project: $($codexHits.Count)"
$codexHits | Sort-Object LastWriteTime -Descending | Select-Object -First 8 |
  ForEach-Object { Write-Host "  $($_.LastWriteTime)  $($_.Name)" }

Section "Gemini CLI"
$gHist = "$env:USERPROFILE\.gemini\history\$marker"
if (Test-Path $gHist) {
  $items = Get-ChildItem $gHist -Recurse -Force -ErrorAction SilentlyContinue
  Write-Host "history entries: $($items.Count)"
  $items | Select-Object -First 10 | ForEach-Object { Write-Host "  $($_.Name) $($_.Length)b" }
} else {
  Write-Host "(no Gemini history dir)"
}
$gSet = "$env:USERPROFILE\.gemini\settings.json"
if (Test-Path $gSet) {
  $raw = Get-Content $gSet -Raw
  $hasAgents = $raw -match 'AGENTS\.md'
  Write-Host "user settings.json exists; context AGENTS.md: $hasAgents (prefer project .gemini/settings.json)"
}

Section "Grok Build"
$gSess = Get-ChildItem "$env:USERPROFILE\.grok\sessions" -Directory -ErrorAction SilentlyContinue |
  Where-Object { $_.Name -match "automation-blocking" -or $_.Name -match "ai-education" }
if ($gSess) {
  foreach ($s in $gSess) {
    Write-Host "session root: $($s.FullName)"
    $ph = Join-Path $s.FullName "prompt_history.jsonl"
    if (Test-Path $ph) {
      Write-Host "  prompt_history lines: $((Get-Content $ph).Count)"
      Get-Content $ph -Tail 5 | ForEach-Object {
        if ($_ -match '"prompt":"([^"]{0,120})') { Write-Host "  recent: $($Matches[1])..." }
      }
    }
  }
} else {
  Write-Host "(no Grok session dir matched)"
}

Section "Repo agent canon"
@(
  "AGENTS.md",
  ".agents/lessons/HARD-WON.md",
  ".agents/lessons/PROVIDER-HISTORY.md",
  "scripts/e2e-docker.sh",
  "sots/38-truth-debt-remediation.md"
) | ForEach-Object {
  $p = Join-Path $Root $_
  if (Test-Path $p) { Write-Host "OK  $_" } else { Write-Host "MISS $_" }
}

Write-Host ""
Write-Host "Done. Durable lessons: .agents/lessons/HARD-WON.md"
Write-Host "Do not commit provider secrets or raw transcripts."
