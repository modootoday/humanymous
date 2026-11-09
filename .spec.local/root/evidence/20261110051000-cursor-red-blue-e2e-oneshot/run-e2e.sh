#!/usr/bin/env bash
set -euo pipefail
export PATH="/c/Program Files/Docker/Docker/resources/bin:$PATH"
export DOCKER_CONTEXT=desktop-linux
cd /c/workspaces/humanymous
echo "[preflight] docker context=$(docker context show)"
docker --context desktop-linux info --format 'ID={{.ID}} OSType={{.OSType}} ServerVersion={{.ServerVersion}}'
# Affinity gate: refuse WSL/other engines
ENGINE_ID=$(docker --context desktop-linux info --format '{{.ID}}')
OSTYPE=$(docker --context desktop-linux info --format '{{.OSType}}')
ENDPOINT=$(docker context inspect desktop-linux --format '{{.Endpoints.docker.Host}}')
echo "[preflight] ENGINE_ID=$ENGINE_ID OSTYPE=$OSTYPE ENDPOINT=$ENDPOINT"
test "$ENGINE_ID" = "578836b2-d812-4a41-91a8-6ed2ae05b7fa"
test "$OSTYPE" = "linux"
test "$ENDPOINT" = "npipe:////./pipe/dockerDesktopLinuxEngine"
# Resource-aware: serialize; no Cap burn
export E2E_PROJECT_NAME=hmn-e2e
export E2E_RUN_ID=20261109T200900Z-hmn-e2e-rb
export E2E_LOG_ROOT="/c/workspaces/humanymous/.spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/runs"
# Full suite per skill (jobs conceptually 1 via single compose project)
bash scripts/e2e-docker.sh
rc=$?
echo "E2E_EXIT=$rc"
exit $rc