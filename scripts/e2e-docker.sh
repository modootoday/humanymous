#!/usr/bin/env bash
# scripts/e2e-docker.sh — authoritative end-to-end suite (Docker only).
#
# All e2e for humanymous must run through the compose stack so the network plane
# (JA3/JA4, multi-subnet correlation) is real. Host/loopback runners are not
# completion authority.
#
# Usage (from repo root):
#   bash scripts/e2e-docker.sh
#   E2E_SKIP_SWARM=1 bash scripts/e2e-docker.sh
#   E2E_SKIP_OVERLAYS=1 bash scripts/e2e-docker.sh   # skip PLAN-08 overlays
#   E2E_KEEP=1 bash scripts/e2e-docker.sh            # leave stack up on success
#   E2E_STEP_TIMEOUT=900 bash scripts/e2e-docker.sh   # per-step hard timeout
#
# Live output is mirrored to .agent-runs/e2e/<run-id>/e2e.log. The current
# phase is always readable from status.json; a heartbeat prints every 30s.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Deterministic project name (NOT a per-run PID). The lab networks pin FIXED subnets (networks.yaml,
# 172.30-32.0.0/24, internal:true) so multi-subnet correlation is real — but fixed subnets are
# SINGLETONS on the daemon: only one stack can hold them. A random per-run name made runs
# non-idempotent (a crashed run left an unfindable stack holding the subnets → next run "Pool
# overlaps"). A stable name means a re-run replaces its own prior stack, and preflight_clean below
# frees the subnets from any OTHER holder — the local equivalent of CI's ephemeral clean runner.
PROJECT="${E2E_PROJECT_NAME:-hmn-e2e}"
PROJECT="$(printf '%s' "$PROJECT" | tr '[:upper:]_' '[:lower:]-' | tr -cd 'a-z0-9-')"
COMPOSE_FILES=(-f deployments/compose.yaml)
if [[ -f deployments/compose.ci.yaml && "${E2E_USE_CI_COMPOSE:-0}" == "1" ]]; then
  COMPOSE_FILES+=(-f deployments/compose.ci.yaml)
fi
COMPOSE=(docker compose -p "$PROJECT" "${COMPOSE_FILES[@]}")

SKIP_SWARM="${E2E_SKIP_SWARM:-0}"
SKIP_OVERLAYS="${E2E_SKIP_OVERLAYS:-0}"
KEEP="${E2E_KEEP:-0}"
STEP_TIMEOUT="${E2E_STEP_TIMEOUT:-900}"
BUILD_TIMEOUT="${E2E_BUILD_TIMEOUT:-600}"
HEARTBEAT_SECONDS="${E2E_HEARTBEAT_SECONDS:-30}"
RUN_ID="${E2E_RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$PROJECT}"
LOG_ROOT="${E2E_LOG_ROOT:-$ROOT/.agent-runs/e2e}"
RUN_DIR="$LOG_ROOT/$RUN_ID"
LOG_FILE="${E2E_LOG_FILE:-$RUN_DIR/e2e.log}"
STATUS_FILE="$RUN_DIR/status.json"
CURRENT_STEP="initializing"
CURRENT_STEP_STARTED="$(date +%s)"
FINAL_STATUS="failed"

mkdir -p "$RUN_DIR"
printf '%s\n' "$LOG_FILE" >"$LOG_ROOT/latest.log.path"
echo "[e2e-docker] live log: $LOG_FILE"
echo "[e2e-docker] live status: $STATUS_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

write_status() {
  local status="$1" rc="${2:-0}" now elapsed tmp
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  elapsed=$(( $(date +%s) - CURRENT_STEP_STARTED ))
  tmp="$STATUS_FILE.tmp.${BASHPID:-$$}"
  printf '{"runId":"%s","project":"%s","status":"%s","phase":"%s","phaseElapsedSeconds":%s,"exitCode":%s,"updatedAt":"%s","log":"%s"}\n' \
    "$RUN_ID" "$PROJECT" "$status" "$CURRENT_STEP" "$elapsed" "$rc" "$now" "$LOG_FILE" >"$tmp"
  mv "$tmp" "$STATUS_FILE"
}

run_step() {
  local name="$1" limit="$2" started rc heartbeat_pid
  shift 2
  CURRENT_STEP="$name"
  CURRENT_STEP_STARTED="$(date +%s)"
  started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  write_status running 0
  echo
  echo "========== [e2e-docker] START $name @ $started (timeout ${limit}s) =========="
  printf '[e2e-docker] COMMAND'
  printf ' %q' "$@"
  printf '\n'
  (
    while sleep "$HEARTBEAT_SECONDS"; do
      write_status running 0
      echo "[e2e-docker] HEARTBEAT phase=$name elapsed=$(( $(date +%s) - CURRENT_STEP_STARTED ))s log=$LOG_FILE"
    done
  ) &
  heartbeat_pid=$!

  set +e
  timeout --signal=TERM --kill-after=15s "${limit}s" "$@"
  rc=$?
  set -e
  kill "$heartbeat_pid" >/dev/null 2>&1 || true
  wait "$heartbeat_pid" 2>/dev/null || true

  if [[ "$rc" -ne 0 ]]; then
    write_status failed "$rc"
    if [[ "$rc" -eq 124 ]]; then
      echo "[e2e-docker] TIMEOUT phase=$name after ${limit}s"
    else
      echo "[e2e-docker] FAIL phase=$name exit=$rc"
    fi
    return "$rc"
  fi
  write_status running 0
  echo "========== [e2e-docker] PASS $name elapsed=$(( $(date +%s) - CURRENT_STEP_STARTED ))s =========="
}

cleanup() {
  if [[ "$KEEP" == "1" ]]; then
    echo "[e2e-docker] E2E_KEEP=1 — leaving stack running"
    return 0
  fi
  echo "[e2e-docker] tearing down stack..."
  timeout 90s "${COMPOSE[@]}" --profile swarm down -v --remove-orphans || true
}
on_exit() {
  local rc=$?
  cleanup
  if [[ "$FINAL_STATUS" == "completed" && "$rc" -eq 0 ]]; then
    write_status completed 0
  elif [[ -f "$STATUS_FILE" ]] && ! grep -q '"status":"failed"' "$STATUS_FILE"; then
    write_status failed "$rc"
  fi
}
trap on_exit EXIT

# preflight_clean frees the fixed lab subnets before we build/up, so a leftover stack (our own prior
# run, a `make up` under the default `deployments` project, or any other holder) can't block us with
# an opaque "Pool overlaps". This gives local the clean-slate guarantee CI gets from an ephemeral
# runner; on an already-clean daemon it is a no-op. Generic: it downs whatever compose project owns a
# `*_lab-[abc]` network, then removes any dangling lab network.
preflight_clean() {
  echo "[e2e-docker] pre-flight: freeing lab subnets (removing any conflicting stack)..."
  timeout 90s "${COMPOSE[@]}" --profile swarm down -v --remove-orphans >/dev/null 2>&1 || true
  local n proj
  for n in $(docker network ls --format '{{.Name}}' 2>/dev/null | grep -E '_lab-[abc]$' || true); do
    proj="$(docker network inspect "$n" --format '{{index .Labels "com.docker.compose.project"}}' 2>/dev/null || true)"
    if [[ -n "$proj" ]]; then
      echo "[e2e-docker] pre-flight: tearing down stack '$proj' holding $n"
      timeout 90s docker compose -p "$proj" -f deployments/compose.yaml down -v --remove-orphans >/dev/null 2>&1 || true
    fi
    docker network rm "$n" >/dev/null 2>&1 || true
  done
  docker network prune -f >/dev/null 2>&1 || true
}
preflight_clean

run_step validate-compose 60 "${COMPOSE[@]}" config -q
run_step build-images "$BUILD_TIMEOUT" "${COMPOSE[@]}" \
  --profile attack --profile gate-test --profile pass-test --profile swarm \
  --progress plain build core gate bots
run_step start-defenders 120 "${COMPOSE[@]}" up -d --no-build core origin gate
run_step attack-catalog "$STEP_TIMEOUT" "${COMPOSE[@]}" run --rm -T bots
run_step attack-assert 120 "${COMPOSE[@]}" run --rm -T attack-assert
run_step gate-conformance 300 "${COMPOSE[@]}" run --rm -T gate-e2e
run_step pass-contract 300 "${COMPOSE[@]}" run --rm -T pass-e2e
run_step restart-core 120 "${COMPOSE[@]}" restart core
run_step pass-wargame 600 "${COMPOSE[@]}" run --rm -T pass-wargame

if [[ "$SKIP_SWARM" != "1" ]]; then
  run_step swarm-reset 120 "${COMPOSE[@]}" run --rm -T swarm-reset
  run_step swarm 300 "${COMPOSE[@]}" --profile swarm up \
    --abort-on-container-failure bot-swarm-a bot-swarm-b bot-swarm-c
  run_step swarm-assert 120 "${COMPOSE[@]}" run --rm -T swarm-assert
else
  echo "[e2e-docker] skip swarm (E2E_SKIP_SWARM=1)"
fi

if [[ "$SKIP_OVERLAYS" != "1" ]]; then
  if [[ -x scripts/ci-overlays.sh ]] || [[ -f scripts/ci-overlays.sh ]]; then
    run_step stop-base-before-overlays 120 "${COMPOSE[@]}" --profile swarm down -v
    run_step overlays "$STEP_TIMEOUT" env \
      E2E_BASE_PROJECT="$PROJECT" \
      E2E_PROJECT_NAME="${PROJECT}-overlay" \
      sh scripts/ci-overlays.sh
  fi
else
  echo "[e2e-docker] skip overlays (E2E_SKIP_OVERLAYS=1)"
fi

FINAL_STATUS="completed"
echo "[e2e-docker] ALL e2e gates passed."
