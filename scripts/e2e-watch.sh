#!/bin/sh
# Follow the most recent Docker E2E run without waiting for the runner process.
set -eu
MODE="${1:-follow}"

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
POINTER="$ROOT/.agent-runs/e2e/latest.log.path"

if [ ! -s "$POINTER" ]; then
  echo "[e2e-watch] no active or previous run: $POINTER is missing" >&2
  exit 1
fi

LOG_FILE="$(cat "$POINTER")"
STATUS_FILE="$(dirname "$LOG_FILE")/status.json"

echo "[e2e-watch] log: $LOG_FILE"
echo "[e2e-watch] status: $STATUS_FILE"
if [ -f "$STATUS_FILE" ]; then
  cat "$STATUS_FILE"
  echo
fi

if [ "$MODE" = "--once" ]; then
  exec tail -n 100 "$LOG_FILE"
fi
exec tail -n 100 -F "$LOG_FILE"
