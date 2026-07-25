#!/usr/bin/env bash
# Coordinate exclusive git writes across multi-provider sessions.
# usage: git-coord.sh preflight|claim|release|status [provider] [session] [note]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
BOARD="$ROOT/scripts/agents/session-board.sh"
CLAIMS="$ROOT/.agents/sessions/claims"
GITOPS="$CLAIMS/git-ops.json"
INDEX_LOCK="$ROOT/.git/index.lock"
TTL_MIN=30
ACTION="${1:-preflight}"

preflight() {
  echo "=== git-coord preflight ==="
  [[ -d .git ]] || { echo "not a git repo" >&2; exit 1; }
  if [[ -f "$INDEX_LOCK" ]]; then
    echo "BLOCKED: .git/index.lock exists" >&2
    exit 1
  fi
  echo "OK  no index.lock"
  if [[ -f "$GITOPS" ]]; then
    mtime=$(stat -c %Y "$GITOPS" 2>/dev/null || stat -f %m "$GITOPS")
    now=$(date +%s)
    age_min=$(( (now - mtime) / 60 ))
    if [[ "$age_min" -le "$TTL_MIN" ]]; then
      echo "BLOCKED: git-ops held (age ${age_min}m)" >&2
      cat "$GITOPS" >&2
      exit 1
    fi
    echo "WARN git-ops STALE (${age_min}m) — FORCE=1 release before reclaim"
  else
    echo "OK  git-ops free"
  fi
  echo "--- git status -sb ---"
  git status -sb
  if git rev-parse --abbrev-ref '@{u}' >/dev/null 2>&1; then
    git fetch --quiet 2>/dev/null || true
    counts=$(git rev-list --left-right --count 'HEAD...@{u}' 2>/dev/null || true)
    if [[ -n "$counts" ]]; then
      ahead=$(echo "$counts" | awk '{print $1}')
      behind=$(echo "$counts" | awk '{print $2}')
      echo "OK  ahead $ahead / behind $behind vs upstream"
      [[ "${behind:-0}" -gt 0 ]] && echo "WARN behind upstream — pull under git-ops before push"
    fi
  fi
  echo "=== preflight done ==="
}

cmd_claim() {
  local provider="${2:-human}" session="${3:-git-$(date +%H%M%S)}"
  FORCE="${FORCE:-0}" \
    "$BOARD" claim git-ops "$provider" "git write transaction" ".git/**" "$session"
  echo "Hold only for the transaction; release immediately after."
}

cmd_release() {
  local note="${2:-}"
  "$BOARD" release git-ops "$note"
}

cmd_status() {
  if [[ -f "$GITOPS" ]]; then
    echo "git-ops: held"; cat "$GITOPS"
  else
    echo "git-ops: free"
  fi
  [[ -f "$INDEX_LOCK" ]] && echo "index.lock: PRESENT" || echo "index.lock: absent"
  git status -sb
}

case "$ACTION" in
  preflight) preflight ;;
  claim) cmd_claim "$@" ;;
  release) cmd_release "$@" ;;
  status) cmd_status ;;
  *) echo "usage: $0 preflight|claim|release|status" >&2; exit 1 ;;
esac
