#Requires -Version 5.1
<#
.SYNOPSIS
  Mirror .agents canon into vendor adapter paths (Claude, Grok, Gemini, Codex).
#>
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

function Ensure-Dir([string]$Path) {
  if (-not (Test-Path $Path)) {
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
  }
}

function Write-GeneratedMarker([string]$Dir, [string]$Kind) {
  Ensure-Dir $Dir
  @"
# GENERATED — do not edit

Mirrored from ``.agents/$Kind`` by ``scripts/agents/sync-adapters``.
Edit ``.agents/`` then re-run sync.
"@ | Set-Content -Path (Join-Path $Dir "GENERATED.md") -Encoding utf8
}

function Copy-Tree([string]$Src, [string]$Dst) {
  if (-not (Test-Path $Src)) {
    Write-Warning "Missing source: $Src"
    return
  }
  Ensure-Dir $Dst
  Get-ChildItem $Dst -Force -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -ne "GENERATED.md" } |
    Remove-Item -Recurse -Force
  Copy-Item -Path (Join-Path $Src "*") -Destination $Dst -Recurse -Force
}

$skillsSrc = Join-Path $Root ".agents\skills"
$rulesSrc = Join-Path $Root ".agents\rules"
$personasSrc = Join-Path $Root ".agents\personas"

foreach ($rel in @(".claude\skills", ".grok\skills", ".gemini\skills", ".codex\skills")) {
  $dst = Join-Path $Root $rel
  Write-GeneratedMarker $dst "skills"
  Copy-Tree $skillsSrc $dst
  Write-Host "Synced skills -> $rel"
}

foreach ($rel in @(".claude\rules", ".grok\rules")) {
  $dst = Join-Path $Root $rel
  Write-GeneratedMarker $dst "rules"
  Copy-Tree $rulesSrc $dst
  Write-Host "Synced rules -> $rel"
}

# Personas -> Claude agents
$agentsDst = Join-Path $Root ".claude\agents"
Write-GeneratedMarker $agentsDst "personas"
if (Test-Path $personasSrc) {
  Get-ChildItem $agentsDst -Force -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -ne "GENERATED.md" } |
    Remove-Item -Recurse -Force
  Get-ChildItem $personasSrc -Filter "*.md" | ForEach-Object {
    $name = $_.BaseName
    $body = Get-Content $_.FullName -Raw
    @"
---
name: $name
description: Project persona $name (from .agents/personas). Use for multi-perspective design panels.
---

$body
"@ | Set-Content -Path (Join-Path $agentsDst "$name.md") -Encoding utf8
  }
  Write-Host "Synced personas -> .claude/agents"
}

# Claude settings (hooks) from fragment
$claudeSettings = Join-Path $Root ".claude\settings.json"
$fragment = Join-Path $Root ".agents\hooks\claude-settings.fragment.json"
if (Test-Path $fragment) {
  Ensure-Dir (Split-Path $claudeSettings)
  $frag = Get-Content $fragment -Raw | ConvertFrom-Json
  $settings = [ordered]@{
    hooks = $frag.hooks
  }
  ($settings | ConvertTo-Json -Depth 10) | Set-Content -Path $claudeSettings -Encoding utf8
  Write-Host "Wrote .claude/settings.json (hooks)"
}

# Grok project hooks
$grokHookSrc = Join-Path $Root ".agents\hooks\grok-project-safety.json"
$grokHookDstDir = Join-Path $Root ".grok\hooks"
if (Test-Path $grokHookSrc) {
  Ensure-Dir $grokHookDstDir
  Copy-Item $grokHookSrc (Join-Path $grokHookDstDir "project-safety.json") -Force
  Write-Host "Synced Grok hooks -> .grok/hooks/project-safety.json"
}

# Lessons (readable from all providers via .agents/; also mirror under .claude for discovery)
$lessonsSrc = Join-Path $Root ".agents\lessons"
if (Test-Path $lessonsSrc) {
  $lessonsDst = Join-Path $Root ".claude\lessons"
  Write-GeneratedMarker $lessonsDst "lessons"
  Copy-Tree $lessonsSrc $lessonsDst
  Write-Host "Synced lessons -> .claude/lessons"
}

Write-Host "Done. Canon remains under .agents/ and AGENTS.md (+ nested AGENTS.md)."
