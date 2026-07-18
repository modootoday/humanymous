#!/usr/bin/env bash
# run-attack.sh — the automation catalog vs the detector engine. LOCAL TARGET
# ONLY: this container is on an internal `lab` network and can reach nothing but
# the detector.
set -uo pipefail

BASE="${HM_BASE:-https://engine:8443}"
RUNS="${HM_RUNS:-1}"

echo "[bots] waiting for the detector engine at ${BASE} ..."
up=""
for i in $(seq 1 60); do
  if node -e "const https=require('https');https.get(process.argv[1]+'/api/session',{rejectUnauthorized:false},r=>{r.resume();process.exit(0)}).on('error',()=>process.exit(1))" "$BASE" 2>/dev/null; then
    up="yes"; echo "[bots] detector engine is up (after $((i*2))s)"; break
  fi
  sleep 2
done
if [ -z "$up" ]; then echo "[bots] ERROR: detector engine never came up at ${BASE}"; exit 1; fi

echo "[bots] launching the automation catalog (HM_RUNS=${RUNS}) ..."
cd /app/test
# Some profiles drive a HEADFUL browser (human baseline, nodriver, xvfb_headful,
# ai_agent, antidetect) to exercise behavioral tells — those need an X server.
# xvfb-run gives the whole catalog a virtual display (headless profiles ignore it).
HM_BASE="$BASE" HM_RUNS="$RUNS" xvfb-run -a node e2e/runner.mjs
code=$?

# Publish the machine-readable results to the mounted artifacts dir, if present.
if [ -d /artifacts ]; then
  cp -f e2e/results.json /artifacts/engine-results.json 2>/dev/null || true
  echo "[bots] wrote /artifacts/engine-results.json"
fi

echo "[bots] attack run complete (runner exit ${code})."
exit "$code"
