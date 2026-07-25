#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root
$failed = 0
$MaxDesc = 1024

function Fail([string]$Msg) { Write-Host "FAIL: $Msg" -ForegroundColor Red; $script:failed++ }
function Ok([string]$Msg) { Write-Host "OK:   $Msg" }

foreach ($f in @("AGENTS.md", "CLAUDE.md", "GEMINI.md", ".agents\manifest.yaml", ".agents\README.md", ".gemini\settings.json")) {
  if (Test-Path (Join-Path $Root $f)) { Ok $f } else { Fail "missing $f" }
}

foreach ($nested in @("internal\AGENTS.md", "cmd\gate\AGENTS.md", "docs\AGENTS.md", "test\AGENTS.md", "web\AGENTS.md")) {
  if (Test-Path (Join-Path $Root $nested)) { Ok "nested $nested" } else { Fail "missing nested $nested" }
}

$settingsPath = Join-Path $Root ".gemini\settings.json"
if (Test-Path $settingsPath) {
  if ((Get-Content $settingsPath -Raw) -match 'AGENTS\.md') { Ok "gemini settings fileName" }
  else { Fail "gemini settings must reference AGENTS.md" }
}

$manifestSkills = @(
  "design-sot", "adversarial-critique", "implement-sot-slice", "red-blue-validate",
  "pass-wargame-round", "docs-from-sot", "cut-release", "review-changes", "handover-pack",
  "optimize-skill-description", "survey-provider-history"
)
foreach ($id in $manifestSkills) {
  $skill = Join-Path $Root ".agents\skills\$id\SKILL.md"
  if (-not (Test-Path $skill)) { Fail "missing skill $id"; continue }
  $text = Get-Content $skill -Raw
  # Frontmatter only (between first two --- fences)
  if ($text -notmatch '(?s)^---\r?\n(.*?)\r?\n---') { Fail "skill $id missing frontmatter"; continue }
  $fm = $Matches[1]
  if ($fm -notmatch '(?m)^name:\s*') { Fail "skill $id missing name:"; continue }
  if ($fm -notmatch '(?m)^description:') { Fail "skill $id missing description:"; continue }
  $desc = ""
  if ($fm -match '(?ms)^description:\s*>\s*\r?\n((?:[ \t]+.+\r?\n?)+)') {
    $desc = ($Matches[1] -replace '(?m)^[ \t]+', '' -replace '\s+', ' ').Trim()
  } elseif ($fm -match '(?m)^description:\s*(.+)$') {
    $desc = $Matches[1].Trim()
  }
  if ($desc.Length -gt $MaxDesc) { Fail "skill $id description length $($desc.Length) > $MaxDesc" }
  else { Ok "skill $id (desc $($desc.Length) chars)" }
}

foreach ($r in @("00-safety.md", "10-go-conventions.md", "20-detection-freeze.md", "30-docs-diataxis.md", "40-ambiguity-ask.md", "50-provider-matrix.md", "60-e2e-docker-only.md", "70-hard-won-lessons.md", "80-truth-debt.md")) {
  if (Test-Path (Join-Path $Root ".agents\rules\$r")) { Ok "rule $r" } else { Fail "missing rule $r" }
}
if (-not (Test-Path (Join-Path $Root "scripts\e2e-docker.sh"))) { Fail "missing scripts/e2e-docker.sh" }
else { Ok "scripts/e2e-docker.sh" }
foreach ($lesson in @("HARD-WON.md", "PROVIDER-HISTORY.md")) {
  if (Test-Path (Join-Path $Root ".agents\lessons\$lesson")) { Ok "lesson $lesson" }
  else { Fail "missing lesson $lesson" }
}
if (-not (Test-Path (Join-Path $Root ".agents\skills\survey-provider-history\SKILL.md"))) {
  Fail "missing skill survey-provider-history"
} else { Ok "skill survey-provider-history" }

foreach ($persona in @("blue-architect", "red-attacker", "sre-ops", "compliance-dpo", "evaluator", "staff-synthesizer")) {
  if (Test-Path (Join-Path $Root ".agents\personas\$persona.md")) { Ok "persona $persona" }
  else { Fail "missing persona $persona" }
}

if (-not (Test-Path (Join-Path $Root ".agents\evals\trigger-queries.json"))) {
  Fail "missing .agents/evals/trigger-queries.json"
} else { Ok "evals trigger-queries.json" }

if (-not (Test-Path (Join-Path $Root "scripts\agents\hooks\pre-tool-guard.py"))) {
  Fail "missing pre-tool-guard.py"
} else { Ok "hook pre-tool-guard.py" }

# Root AGENTS size budget (~150 lines recommended)
$rootLines = (Get-Content (Join-Path $Root "AGENTS.md")).Count
if ($rootLines -gt 200) { Fail "AGENTS.md has $rootLines lines (keep ≤200)" }
else { Ok "AGENTS.md line count $rootLines" }

$canonSkills = Join-Path $Root ".agents\skills"
if (Test-Path $canonSkills) {
  $expected = (Get-ChildItem $canonSkills -Directory).Name | Sort-Object
  foreach ($vendor in @(".claude\skills", ".grok\skills", ".gemini\skills", ".codex\skills")) {
    $vpath = Join-Path $Root $vendor
    if (-not (Test-Path $vpath)) {
      Write-Host "SKIP: $vendor (run sync-adapters)"
      continue
    }
    $got = Get-ChildItem $vpath -Directory -ErrorAction SilentlyContinue | ForEach-Object { $_.Name } | Sort-Object
    $missing = Compare-Object $expected $got | Where-Object { $_.SideIndicator -eq "<=" }
    if ($missing) { Fail "$vendor missing: $($missing.InputObject -join ', ')" }
    else { Ok "$vendor parity" }
  }
}

foreach ($stub in @("CLAUDE.md", "GEMINI.md")) {
  if ((Get-Content (Join-Path $Root $stub) -Raw) -match 'AGENTS\.md') { Ok "$stub -> AGENTS.md" }
  else { Fail "$stub should reference AGENTS.md" }
}

# Progressive disclosure table present
if ((Get-Content (Join-Path $Root "AGENTS.md") -Raw) -match 'Progressive disclosure') {
  Ok "AGENTS.md progressive disclosure router"
} else { Fail "AGENTS.md missing progressive disclosure section" }

if ($failed -gt 0) {
  Write-Host "`n$failed check(s) failed." -ForegroundColor Red
  exit 1
}
Write-Host "`nAll agent-layout checks passed." -ForegroundColor Green
exit 0
