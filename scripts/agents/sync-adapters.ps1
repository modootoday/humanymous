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

function Write-LfText([string]$Path, [string]$Content) {
  # Generated adapter files must be byte-identical to the bash variant's output
  # (CI runs sync-adapters.sh and diffs), so pin LF regardless of core.autocrlf.
  [System.IO.File]::WriteAllText($Path, ($Content -replace "`r`n", "`n"),
    (New-Object System.Text.UTF8Encoding $false))
}

function Write-GeneratedMarker([string]$Dir, [string]$Kind) {
  Ensure-Dir $Dir
  Write-LfText (Join-Path $Dir "GENERATED.md") @"
# GENERATED — do not edit

Mirrored from ``.agents/$Kind`` by ``scripts/agents/sync-adapters``.
Edit ``.agents/`` then re-run sync.

"@
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
    # Must stay byte-identical to sync-adapters.sh (CI runs the bash variant and
    # diffs the result): LF endings, and no newline beyond the body's own.
    $content = @"
---
name: $name
description: Project persona $name (from .agents/personas). Use for multi-perspective design panels.
---

$body
"@
    Write-LfText (Join-Path $agentsDst "$name.md") $content
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

# Codex project hooks
$codexHookSrc = Join-Path $Root ".agents\hooks\codex-hooks.json"
if (Test-Path $codexHookSrc) {
  $codexHookDstDir = Join-Path $Root ".codex"
  Ensure-Dir $codexHookDstDir
  Copy-Item $codexHookSrc (Join-Path $codexHookDstDir "hooks.json") -Force
  Write-Host "Synced Codex hooks -> .codex/hooks.json"
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
