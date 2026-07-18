#!/usr/bin/env bash
# run-gate-e2e.sh — self-contained Gate conformance run. The e2e harness
# (test/gate/e2e.mjs) stands up its OWN instrumented upstream on :9000 and
# asserts on proxy behavior + edge enforcement + origin cloaking, so the Gate
# and that upstream must share one loopback. We therefore run a throwaway Gate
# here, on this container's 127.0.0.1, exactly mirroring the local dev run.
set -uo pipefail

TOKENS="auditor:e2e-auditor-token,operator:e2e-operator-token,approver:e2e-approver-token,dpo:e2e-dpo-token"

echo "[gate-e2e] starting a throwaway Gate on loopback ..."
HMN_ADMIN_TOKENS="$TOKENS" HMN_ALLOW_DEV_TOKENS=1 /app/bin/gate \
  -addr :8444 -admin-addr :8445 \
  -upstream http://127.0.0.1:9000 \
  -origin-key demo-origin-secret \
  -node gate-e2e >/tmp/gate.log 2>&1 &
GATE_PID=$!

# Wait for the edge listener to accept TLS.
up=""
for i in $(seq 1 30); do
  if node -e "const https=require('https');https.get('https://127.0.0.1:8444/health',{rejectUnauthorized:false},r=>{r.resume();process.exit(0)}).on('error',()=>process.exit(1))" 2>/dev/null; then
    up="yes"; echo "[gate-e2e] Gate edge is up (after $((i))s)"; break
  fi
  sleep 1
done
if [ -z "$up" ]; then echo "[gate-e2e] ERROR: Gate never came up"; cat /tmp/gate.log; kill $GATE_PID 2>/dev/null; exit 1; fi

cd /app/test
HM_PROXY="https://127.0.0.1:8444" HM_ADMIN="https://127.0.0.1:8445" HM_ORIGIN_KEY="demo-origin-secret" \
  node gate/e2e.mjs
code=$?

if [ -d /artifacts ]; then cp -f /tmp/gate.log /artifacts/gate-e2e.log 2>/dev/null || true; fi
kill $GATE_PID 2>/dev/null || true
echo "[gate-e2e] conformance complete (exit ${code})."
exit "$code"
