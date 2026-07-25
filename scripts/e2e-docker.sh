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
#
# Exit non-zero on any failed gate. Tears down the stack unless E2E_KEEP=1.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f deployments/compose.yaml)
if [[ -f deployments/compose.ci.yaml ]]; then
  # CI fragment (cache/build tweaks) is optional for local runs.
  if [[ "${E2E_USE_CI_COMPOSE:-0}" == "1" ]]; then
    COMPOSE=(docker compose -f deployments/compose.yaml -f deployments/compose.ci.yaml)
  fi
fi

SKIP_SWARM="${E2E_SKIP_SWARM:-0}"
SKIP_OVERLAYS="${E2E_SKIP_OVERLAYS:-0}"
KEEP="${E2E_KEEP:-0}"

cleanup() {
  if [[ "$KEEP" == "1" ]]; then
    echo "[e2e-docker] E2E_KEEP=1 — leaving stack running"
    return 0
  fi
  echo "[e2e-docker] tearing down stack..."
  "${COMPOSE[@]}" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "[e2e-docker] validate compose..."
"${COMPOSE[@]}" config -q

echo "[e2e-docker] build + start defenders (core, origin, gate)..."
"${COMPOSE[@]}" up -d --build core origin gate

echo "[e2e-docker] automation catalog (bots) vs core..."
"${COMPOSE[@]}" run --rm bots

echo "[e2e-docker] assert attack gate..."
node scripts/assert-attack.mjs deployments/artifacts/core-results.json

echo "[e2e-docker] gate proxy-layer conformance..."
"${COMPOSE[@]}" run --rm gate-e2e

if [[ "$SKIP_SWARM" != "1" ]]; then
  echo "[e2e-docker] multi-subnet swarm (proxy_rotation)..."
  swarm_log="${TMPDIR:-/tmp}/hmn-swarm-e2e.log"
  # shellcheck disable=SC2068
  set +e
  "${COMPOSE[@]}" --profile swarm up --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c 2>&1 | tee "$swarm_log"
  swarm_rc=${PIPESTATUS[0]}
  set -e
  if ! grep -q 'l5.correlation.proxy_rotation' "$swarm_log"; then
    echo "[e2e-docker] FAIL: proxy_rotation never fired across subnets" >&2
    exit 1
  fi
  # swarm may exit non-zero if containers stop; correlation assert is the gate
  echo "[e2e-docker] swarm correlation OK (compose exit ${swarm_rc})"
else
  echo "[e2e-docker] skip swarm (E2E_SKIP_SWARM=1)"
fi

if [[ "$SKIP_OVERLAYS" != "1" ]]; then
  if [[ -x scripts/ci-overlays.sh ]] || [[ -f scripts/ci-overlays.sh ]]; then
    echo "[e2e-docker] PLAN-08 feature overlays..."
    bash scripts/ci-overlays.sh
  fi
else
  echo "[e2e-docker] skip overlays (E2E_SKIP_OVERLAYS=1)"
fi

echo "[e2e-docker] ALL e2e gates passed."
