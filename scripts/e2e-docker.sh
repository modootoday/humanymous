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
# Cap=0 Desktop EXIT137 containment: shm/mem_limit via lab overlay (compose run
# on Desktop lacks --shm-size/--memory flags). Disable with E2E_DESKTOP_PRESSURE=0.
if [[ "${E2E_DESKTOP_PRESSURE:-1}" != "0" && -f deployments/lab/compose.e2e-desktop-pressure.yaml ]]; then
  COMPOSE_FILES+=(-f deployments/lab/compose.e2e-desktop-pressure.yaml)
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
PRESSURE_FILE="$RUN_DIR/pressure-guard-bash.json"
CURRENT_STEP="initializing"
CURRENT_STEP_STARTED="$(date +%s)"
FINAL_STATUS="failed"
# Cap=0 Desktop EXIT137 containment: shm + bots cgroup memory (compose run flags).
BOTS_SHM_SIZE="${E2E_BOTS_SHM_SIZE:-1g}"
BOTS_MEMORY="${E2E_BOTS_MEMORY:-12g}"
PRESSURE_GUARD="${E2E_PRESSURE_GUARD:-1}"
if [[ "$SKIP_SWARM" == "1" ]]; then
  MIN_VM_MEM_AVAILABLE_MB="${E2E_MIN_VM_MEM_AVAILABLE_MB:-12288}"
else
  MIN_VM_MEM_AVAILABLE_MB="${E2E_MIN_VM_MEM_AVAILABLE_MB:-16384}"
fi
MIN_VM_SWAP_FREE_MB="${E2E_MIN_VM_SWAP_FREE_MB:-512}"
MIN_VM_MEM_TOTAL_MB="${E2E_MIN_VM_MEM_TOTAL_MB:-16384}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

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

# Cap=0 Desktop VM memory/pressure guard (fail-closed). Prefer EXIT 2 refuse over
# mid-catalog bots SIGKILL (EXIT 137) when MemAvailable/swap are already low.
# Never invents pass_product_clean_room.
pressure_guard() {
  local phase="$1"
  if [[ "$PRESSURE_GUARD" == "0" ]]; then
    echo "[e2e-docker] pressure-guard skipped (E2E_PRESSURE_GUARD=0) phase=$phase"
    return 0
  fi
  local meminfo="" img="" candidate
  for candidate in humanymous/bots:local busybox:1.36 busybox:latest alpine:3.20; do
    if docker image inspect "$candidate" >/dev/null 2>&1; then
      if meminfo="$(docker run --rm --network none --entrypoint cat "$candidate" /proc/meminfo 2>/dev/null)"; then
        img="$candidate"
        break
      fi
    fi
  done
  if [[ -z "$meminfo" && -r /proc/meminfo ]]; then
    meminfo="$(cat /proc/meminfo)"
    img="host:/proc/meminfo"
  fi
  local mem_total_kb mem_avail_kb swap_free_kb swap_total_kb
  mem_total_kb="$(printf '%s\n' "$meminfo" | awk '/^MemTotal:/ {print $2; exit}')"
  mem_avail_kb="$(printf '%s\n' "$meminfo" | awk '/^MemAvailable:/ {print $2; exit}')"
  swap_free_kb="$(printf '%s\n' "$meminfo" | awk '/^SwapFree:/ {print $2; exit}')"
  swap_total_kb="$(printf '%s\n' "$meminfo" | awk '/^SwapTotal:/ {print $2; exit}')"
  local vm_total_mb=0 vm_avail_mb=0 vm_swap_free_mb=0 vm_swap_total_mb=0
  [[ -n "$mem_total_kb" ]] && vm_total_mb=$((mem_total_kb / 1024))
  [[ -n "$mem_avail_kb" ]] && vm_avail_mb=$((mem_avail_kb / 1024))
  [[ -n "$swap_free_kb" ]] && vm_swap_free_mb=$((swap_free_kb / 1024))
  [[ -n "$swap_total_kb" ]] && vm_swap_total_mb=$((swap_total_kb / 1024))

  local reasons=()
  if [[ -z "$meminfo" ]]; then reasons+=("meminfo_unreadable"); fi
  if [[ -z "$mem_avail_kb" ]]; then reasons+=("MemAvailable_missing")
  elif [[ "$vm_avail_mb" -lt "$MIN_VM_MEM_AVAILABLE_MB" ]]; then
    reasons+=("MemAvailable_MB=${vm_avail_mb}<min=${MIN_VM_MEM_AVAILABLE_MB}")
  fi
  if [[ -z "$mem_total_kb" ]]; then reasons+=("MemTotal_missing")
  elif [[ "$vm_total_mb" -lt "$MIN_VM_MEM_TOTAL_MB" ]]; then
    reasons+=("MemTotal_MB=${vm_total_mb}<min=${MIN_VM_MEM_TOTAL_MB}")
  fi
  if [[ -z "$swap_free_kb" ]]; then reasons+=("SwapFree_missing")
  elif [[ "$vm_swap_free_mb" -lt "$MIN_VM_SWAP_FREE_MB" ]]; then
    reasons+=("SwapFree_MB=${vm_swap_free_mb}<min=${MIN_VM_SWAP_FREE_MB}")
  fi

  local result="PASS"
  if [[ "${#reasons[@]}" -gt 0 ]]; then result="REFUSE"; fi
  local reason_json="[]"
  if [[ "${#reasons[@]}" -gt 0 ]]; then
    reason_json="$(printf '%s\n' "${reasons[@]}" | awk 'BEGIN{printf "["} {gsub(/"/,"\\\""); printf sep "\"" $0 "\""; sep=","} END{printf "]"}')"
  fi
  printf '{"schema":"humanymous.e2e-desktop-pressure-guard-bash/1","phase":"%s","result":"%s","pass_product_clean_room":0,"product":0,"probe_image":"%s","thresholds_mb":{"min_vm_mem_available":%s,"min_vm_swap_free":%s,"min_vm_mem_total":%s},"observed_mb":{"vm_mem_total":%s,"vm_mem_available":%s,"vm_swap_total":%s,"vm_swap_free":%s},"refuse_reasons":%s,"bots_shm_size":"%s","bots_memory":"%s","updatedAt":"%s"}\n' \
    "$phase" "$result" "$img" \
    "$MIN_VM_MEM_AVAILABLE_MB" "$MIN_VM_SWAP_FREE_MB" "$MIN_VM_MEM_TOTAL_MB" \
    "$vm_total_mb" "$vm_avail_mb" "$vm_swap_total_mb" "$vm_swap_free_mb" \
    "$reason_json" "$BOTS_SHM_SIZE" "$BOTS_MEMORY" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    >"$PRESSURE_FILE"

  if [[ "$result" != "PASS" ]]; then
    echo "[e2e-docker] REFUSE pressure-guard phase=$phase reasons=${reasons[*]} (receipt=$PRESSURE_FILE)"
    echo "[e2e-docker] Fail-closed: raise Desktop VM memory/swap or free pressure before full e2e. product=0."
    write_status failed 2
    exit 2
  fi
  echo "[e2e-docker] pressure-guard PASS phase=$phase vmAvailMB=$vm_avail_mb swapFreeMB=$vm_swap_free_mb botsMem=$BOTS_MEMORY shm=$BOTS_SHM_SIZE"
}

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
pressure_guard preflight
preflight_clean

run_step validate-compose 60 "${COMPOSE[@]}" config -q
run_step build-images "$BUILD_TIMEOUT" "${COMPOSE[@]}" \
  --profile attack --profile gate-test --profile pass-test --profile swarm \
  --progress plain build core gate bots
run_step start-defenders 120 "${COMPOSE[@]}" up -d --no-build core origin gate

# Cap=0 Desktop flake guard: rapid preflight down -v + up can leave Docker
# embedded DNS and core RIT briefly unstable. Observed failure mode on
# desktop-linux: human.mjs DENY via HR-16 (l5.rit.header_tampered) on a cold
# core that only answered HTTPS for ~2s, then mid-catalog SERVFAIL
# (lookup core on 127.0.0.11:53: server misbehaving). Do NOT touch scoring
# (detection freeze). Require consecutive HTTPS successes from the bots/lab-a
# network namespace, then a short settle pause before the catalog.
CURRENT_STEP="settle-defenders"
CURRENT_STEP_STARTED="$(date +%s)"
write_status running 0
echo
echo "========== [e2e-docker] START settle-defenders @ $(date -u +%Y-%m-%dT%H:%M:%SZ) =========="
settle_streak=0
settle_ok=0
for settle_i in $(seq 1 30); do
  if "${COMPOSE[@]}" --profile attack run --rm -T --no-deps --entrypoint node bots \
    -e 'const https=require("https");https.get("https://core:8443/api/session",{rejectUnauthorized:false},r=>{r.resume();process.exit((r.statusCode||500)<500?0:1)}).on("error",()=>process.exit(1));' \
    >/dev/null 2>&1; then
    settle_streak=$((settle_streak + 1))
    echo "[e2e-docker] settle probe #${settle_i} ok streak=${settle_streak}"
    if [[ "$settle_streak" -ge 3 ]]; then
      settle_ok=1
      break
    fi
  else
    settle_streak=0
    echo "[e2e-docker] settle probe #${settle_i} fail; reset streak"
  fi
  sleep 2
done
if [[ "$settle_ok" -ne 1 ]]; then
  write_status failed 1
  echo "[e2e-docker] FAIL settle-defenders: never reached 3 consecutive HTTPS successes"
  exit 1
fi
echo "[e2e-docker] settle-defenders: stable; pausing 8s for Desktop DNS/RIT"
sleep 8
echo "========== [e2e-docker] PASS settle-defenders elapsed=$(( $(date +%s) - CURRENT_STEP_STARTED ))s =========="

# Re-check after build/defenders: prefer refuse over mid-catalog EXIT137.
pressure_guard before-attack-catalog

# Resource bounds come from deployments/lab/compose.e2e-desktop-pressure.yaml
# (compose run on Desktop rejects --shm-size/--memory CLI flags).
run_step attack-catalog "$STEP_TIMEOUT" "${COMPOSE[@]}" run --rm -T bots
run_step attack-assert 120 "${COMPOSE[@]}" run --rm -T attack-assert
run_step gate-conformance 300 "${COMPOSE[@]}" run --rm -T gate-e2e
run_step pass-contract 300 "${COMPOSE[@]}" run --rm -T pass-e2e
run_step restart-core 120 "${COMPOSE[@]}" restart core
run_step pass-wargame 600 "${COMPOSE[@]}" run --rm -T pass-wargame

if [[ "$SKIP_SWARM" != "1" ]]; then
  pressure_guard before-swarm
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
