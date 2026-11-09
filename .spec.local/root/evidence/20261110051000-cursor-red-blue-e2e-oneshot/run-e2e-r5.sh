#!/usr/bin/env bash
set -euo pipefail
export PATH="/c/Program Files/Docker/Docker/resources/bin:$PATH"
export DOCKER_CONTEXT=desktop-linux
cd /c/workspaces/humanymous
ENGINE_ID=$(docker --context desktop-linux info --format '{{.ID}}')
OSTYPE=$(docker --context desktop-linux info --format '{{.OSType}}')
ENDPOINT=$(docker context inspect desktop-linux --format '{{.Endpoints.docker.Host}}')
echo "[preflight] ENGINE_ID=$ENGINE_ID OSTYPE=$OSTYPE ENDPOINT=$ENDPOINT tip=$(git rev-parse --short HEAD)"
test "$ENGINE_ID" = "578836b2-d812-4a41-91a8-6ed2ae05b7fa"
test "$OSTYPE" = "linux"
test "$ENDPOINT" = "npipe:////./pipe/dockerDesktopLinuxEngine"
export E2E_PROJECT_NAME=hmn-e2e
export E2E_RUN_ID=20261109T203300Z-hmn-e2e-rb-r5
export E2E_LOG_ROOT="/c/workspaces/humanymous/.spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/runs"
export E2E_BUILD_TIMEOUT=1800
export E2E_STEP_TIMEOUT=1200
bash scripts/e2e-docker.sh
rc=$?
echo "E2E_EXIT=$rc"
exit $rc