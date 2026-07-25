#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
failed=0
MAX_DESC=1024

fail() { echo "FAIL: $*" >&2; failed=$((failed + 1)); }
ok() { echo "OK:   $*"; }

for f in AGENTS.md CLAUDE.md GEMINI.md .agents/manifest.yaml .agents/README.md .gemini/settings.json; do
  [[ -e "$f" ]] && ok "$f" || fail "missing $f"
done

for nested in internal/AGENTS.md cmd/gate/AGENTS.md docs/AGENTS.md test/AGENTS.md web/AGENTS.md; do
  [[ -f "$nested" ]] && ok "nested $nested" || fail "missing nested $nested"
done

if [[ -f .gemini/settings.json ]]; then
  grep -q 'AGENTS.md' .gemini/settings.json && ok "gemini settings fileName" || fail "gemini settings must reference AGENTS.md"
fi

skills=(
  design-sot adversarial-critique implement-sot-slice red-blue-validate
  pass-wargame-round docs-from-sot cut-release review-changes handover-pack
  optimize-skill-description survey-provider-history coordinate-sessions
)
for id in "${skills[@]}"; do
  f=".agents/skills/$id/SKILL.md"
  if [[ ! -f "$f" ]]; then fail "missing skill $id"; continue; fi
  # Frontmatter between first two --- lines only
  fm=$(awk 'BEGIN{n=0} /^---/{n++; next} n==1{print} n>=2{exit}' "$f")
  if [[ -z "$fm" ]]; then fail "skill $id missing frontmatter"; continue; fi
  echo "$fm" | grep -qE '^name:' || { fail "skill $id missing name:"; continue; }
  echo "$fm" | grep -qE '^description:' || { fail "skill $id missing description:"; continue; }
  desc=$(echo "$fm" | awk '
    BEGIN{p=0}
    /^description:[[:space:]]*>/ {p=1; next}
    /^description:[[:space:]]*/ {sub(/^description:[[:space:]]*/,""); print; exit}
    p && /^[a-zA-Z0-9_-]+:/ {exit}
    p {print}
  ' | tr -s ' \n' ' ' | sed 's/^ *//;s/ *$//')
  len=${#desc}
  if [[ "$len" -gt "$MAX_DESC" ]]; then fail "skill $id description length $len > $MAX_DESC"
  else ok "skill $id (desc ~$len chars)"; fi
done

for r in 00-safety.md 10-go-conventions.md 20-detection-freeze.md 30-docs-diataxis.md 40-ambiguity-ask.md 50-provider-matrix.md 60-e2e-docker-only.md 70-hard-won-lessons.md 80-truth-debt.md 90-session-overlap.md 91-git-contention.md 92-git-commit-attribution.md; do
  [[ -f ".agents/rules/$r" ]] && ok "rule $r" || fail "missing rule $r"
done
for sf in README.md LANES.md PROTOCOL.md GIT-PROTOCOL.md COMMIT-CONVENTION.md ACTIVE.example.md; do
  [[ -f ".agents/sessions/$sf" ]] && ok "sessions $sf" || fail "missing sessions/$sf"
done
[[ -f scripts/agents/session-board.ps1 ]] && ok "session-board.ps1" || fail "missing session-board.ps1"
[[ -f scripts/agents/git-coord.ps1 ]] && ok "git-coord.ps1" || fail "missing git-coord.ps1"
[[ -f scripts/agents/git-commit.sh ]] && ok "git-commit.sh" || fail "missing git-commit.sh"
[[ -f scripts/agents/check-commit-attribution.ps1 ]] && ok "check-commit-attribution.ps1" || fail "missing check-commit-attribution.ps1"
[[ -f .agents/skills/coordinate-sessions/SKILL.md ]] && ok "skill coordinate-sessions" || fail "missing coordinate-sessions"
[[ -f scripts/e2e-docker.sh ]] && ok "scripts/e2e-docker.sh" || fail "missing scripts/e2e-docker.sh"
for lesson in HARD-WON.md PROVIDER-HISTORY.md; do
  [[ -f ".agents/lessons/$lesson" ]] && ok "lesson $lesson" || fail "missing lesson $lesson"
done
[[ -f .agents/skills/survey-provider-history/SKILL.md ]] && ok "skill survey-provider-history" || fail "missing survey-provider-history"

for p in blue-architect red-attacker sre-ops compliance-dpo evaluator staff-synthesizer; do
  [[ -f ".agents/personas/$p.md" ]] && ok "persona $p" || fail "missing persona $p"
done

[[ -f .agents/evals/trigger-queries.json ]] && ok "evals trigger-queries.json" || fail "missing evals"
[[ -f scripts/agents/hooks/pre-tool-guard.py ]] && ok "hook pre-tool-guard.py" || fail "missing pre-tool-guard.py"

lines=$(wc -l < AGENTS.md | tr -d ' ')
if [[ "$lines" -gt 200 ]]; then fail "AGENTS.md has $lines lines (keep ≤200)"; else ok "AGENTS.md line count $lines"; fi

grep -q 'Progressive disclosure' AGENTS.md && ok "AGENTS.md progressive disclosure router" || fail "AGENTS.md missing progressive disclosure"

if [[ -d .agents/skills ]]; then
  mapfile -t expected < <(find .agents/skills -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort)
  for vendor in .claude/skills .grok/skills .gemini/skills .codex/skills; do
    if [[ ! -d "$vendor" ]]; then
      echo "SKIP: $vendor (run sync-adapters)"
      continue
    fi
    mapfile -t got < <(find "$vendor" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort)
    missing=""
    for e in "${expected[@]}"; do
      found=0
      for g in "${got[@]:-}"; do [[ "$g" == "$e" ]] && found=1 && break; done
      [[ $found -eq 0 ]] && missing+="$e "
    done
    [[ -n "$missing" ]] && fail "$vendor missing: $missing" || ok "$vendor parity"
  done
fi

for stub in CLAUDE.md GEMINI.md; do
  grep -q 'AGENTS.md' "$stub" && ok "$stub -> AGENTS.md" || fail "$stub should reference AGENTS.md"
done

if [[ "$failed" -gt 0 ]]; then
  echo; echo "$failed check(s) failed." >&2
  exit 1
fi
echo; echo "All agent-layout checks passed."
exit 0
