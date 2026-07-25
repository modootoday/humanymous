#!/usr/bin/env bash
# scripts/e2e.sh — thin wrapper to the authoritative Docker e2e suite.
#
# Historically this script built a host binary and ran the catalog on loopback.
# That path cannot exercise multi-subnet correlation or realistic L5 topology.
# All e2e completion authority is Docker compose (see scripts/e2e-docker.sh).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "[e2e] forwarding to Docker e2e (scripts/e2e-docker.sh)..."
echo "[e2e] host/loopback catalog runs are not completion authority."
exec bash scripts/e2e-docker.sh "$@"
