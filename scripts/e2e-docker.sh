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

PROJECT="${E2E_PROJECT_NAME:-hmn-e2e-${CI_RUN_ID:-$$}}"
PROJECT="$(printf '%s' "$PROJECT" | tr '[:upper:]_' '[:lower:]-' | tr -cd 'a-z0-9-')"
COMPOSE_FILES=(-f deployments/compose.yaml)
if [[ -f deployments/compose.ci.yaml && "${E2E_USE_CI_COMPOSE:-0}" == "1" ]]; then
  COMPOSE_FILES+=(-f deployments/compose.ci.yaml)
fi
COMPOSE=(docker compose -p "$PROJECT" "${COMPOSE_FILES[@]}")

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
"${COMPOSE[@]}" run --rm attack-assert

echo "[e2e-docker] gate proxy-layer conformance..."
"${COMPOSE[@]}" run --rm gate-e2e

echo "[e2e-docker] Pass contract..."
"${COMPOSE[@]}" run --rm pass-e2e
"${COMPOSE[@]}" restart core
echo "[e2e-docker] Pass wargame..."
"${COMPOSE[@]}" run --rm pass-wargame

if [[ "$SKIP_SWARM" != "1" ]]; then
  echo "[e2e-docker] multi-subnet swarm (proxy_rotation)..."
  "${COMPOSE[@]}" run --rm swarm-reset
  # Prefer --abort-on-container-exit (widely available); failure alias may not exist on all Compose versions.
  "${COMPOSE[@]}" --profile swarm up --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c
  "${COMPOSE[@]}" run --rm swarm-assert
  echo "[e2e-docker] swarm correlation OK"
else
  echo "[e2e-docker] skip swarm (E2E_SKIP_SWARM=1)"
fi

if [[ "$SKIP_OVERLAYS" != "1" ]]; then
  if [[ -x scripts/ci-overlays.sh ]] || [[ -f scripts/ci-overlays.sh ]]; then
    echo "[e2e-docker] PLAN-08 feature overlays..."
    "${COMPOSE[@]}" down -v
    E2E_PROJECT_NAME="${PROJECT}-overlay" sh scripts/ci-overlays.sh
  fi
else
  echo "[e2e-docker] skip overlays (E2E_SKIP_OVERLAYS=1)"
fi

echo "[e2e-docker] ALL e2e gates passed."
