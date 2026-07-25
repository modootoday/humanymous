#!/usr/bin/env bash
# Multi-provider session board: list | claim | release | init
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BOARD="$ROOT/.agents/sessions"
ACTIVE="$BOARD/ACTIVE.md"
EXAMPLE="$BOARD/ACTIVE.example.md"
CLAIMS="$BOARD/claims"
TTL_HOURS=4
ACTION="${1:-list}"

VALID="detection-core gate-edge pass red-catalog e2e-infra docs agents-meta sot-plan release git-ops misc"

ensure_board() {
  mkdir -p "$CLAIMS"
  if [[ ! -f "$ACTIVE" ]]; then
    if [[ -f "$EXAMPLE" ]]; then
      cp "$EXAMPLE" "$ACTIVE"
      echo "Seeded ACTIVE.md from ACTIVE.example.md"
    else
      cat >"$ACTIVE" <<EOF
# ACTIVE session board
Last updated: $(date -Iminutes)
## Claims
| Lane | Provider | Session | Goal | Paths | Since | Status |
|------|----------|---------|------|-------|-------|--------|
| — | — | — | — | — | — | free |
## Handovers (incomplete work)
EOF
    fi
  fi
}

is_stale() {
  local f="$1"
  [[ -f "$f" ]] || return 1
  local now mtime age
  now=$(date +%s)
  mtime=$(stat -c %Y "$f" 2>/dev/null || stat -f %m "$f")
  age=$(( (now - mtime) / 3600 ))
  [[ "$age" -gt "$TTL_HOURS" ]]
}

cmd_list() {
  ensure_board
  local any=0
  for L in $VALID; do
    f="$CLAIMS/$L.json"
    [[ -f "$f" ]] || continue
    any=1
    mark="active"
    is_stale "$f" && mark="STALE"
    echo "[$mark] $L"
    # portable-ish json peek
    python3 -c "import json;d=json.load(open('$f'));print('  provider=',d.get('provider'));print('  session=',d.get('session'));print('  goal=',d.get('goal'));print('  paths=',d.get('paths'));print('  since=',d.get('since'))" 2>/dev/null \
      || cat "$f"
  done
  [[ "$any" -eq 1 ]] || echo "(no active claims — all lanes free)"
}

cmd_claim() {
  ensure_board
  local lane="${2:-}" provider="${3:-}" goal="${4:-}" paths="${5:-}" session="${6:-local-$(date +%H%M%S)}"
  force="${FORCE:-0}"
  [[ -n "$lane" && -n "$provider" && -n "$goal" ]] || {
    echo "usage: $0 claim <lane> <provider> <goal> [paths] [session]" >&2
    exit 1
  }
  echo " $VALID " | grep -q " $lane " || { echo "unknown lane: $lane" >&2; exit 1; }
  f="$CLAIMS/$lane.json"
  if [[ -f "$f" && "$force" != "1" ]]; then
    if ! is_stale "$f"; then
      echo "lane $lane is held (use FORCE=1 if abandoned)" >&2
      cat "$f" >&2
      exit 1
    fi
    echo "replacing STALE claim on $lane"
  fi
  if [[ -z "$paths" ]]; then paths="(see LANES.md for $lane)"; fi
  python3 - <<PY
import json, datetime
from pathlib import Path
p = Path(r"$f")
p.write_text(json.dumps({
  "lane": "$lane",
  "provider": "$provider",
  "session": "$session",
  "goal": """$goal""",
  "paths": """$paths""",
  "since": datetime.datetime.now().isoformat(timespec="seconds"),
  "status": "active",
}, indent=2), encoding="utf-8")
print("claimed", "$lane")
PY
}

cmd_release() {
  ensure_board
  local lane="${2:-}" note="${3:-}"
  [[ -n "$lane" ]] || { echo "usage: $0 release <lane> [note]" >&2; exit 1; }
  f="$CLAIMS/$lane.json"
  if [[ -f "$f" ]]; then
    if [[ -n "$note" ]]; then
      {
        echo ""
        echo "### $(date -Iminutes) — release $lane"
        echo "- **Note:** $note"
      } >>"$ACTIVE"
    fi
    rm -f "$f"
    echo "released $lane"
  else
    echo "lane $lane already free"
  fi
}

case "$ACTION" in
  init) ensure_board; echo "board ready $ACTIVE" ;;
  list) cmd_list ;;
  claim) cmd_claim "$@" ;;
  release) cmd_release "$@" ;;
  *) echo "usage: $0 list|claim|release|init" >&2; exit 1 ;;
esac
