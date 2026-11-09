#!/usr/bin/env bash
set -euo pipefail
export PATH="/c/Program Files/Docker/Docker/resources/bin:$PATH"
export DOCKER_CONTEXT=desktop-linux
cd /c/workspaces/humanymous
LOG="/c/workspaces/humanymous/.spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/console-r7.log"
RCFILE="/c/workspaces/humanymous/.spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/r7.exit"
: >"$LOG"
echo "starting" >"$RCFILE"
ENGINE_ID=$(docker --context desktop-linux info --format '{{.ID}}')
test "$ENGINE_ID" = "578836b2-d812-4a41-91a8-6ed2ae05b7fa"
export E2E_PROJECT_NAME=hmn-e2e
export E2E_RUN_ID=20261109T204000Z-hmn-e2e-rb-r7
export E2E_LOG_ROOT="/c/workspaces/humanymous/.spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/runs"
export E2E_BUILD_TIMEOUT=1800
export E2E_STEP_TIMEOUT=1200
set +e
bash scripts/e2e-docker.sh >"$LOG" 2>&1
rc=$?
set -e
echo "$rc" >"$RCFILE"
echo "E2E_EXIT=$rc" >>"$LOG"
exit "$rc"