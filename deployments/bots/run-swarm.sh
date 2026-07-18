#!/usr/bin/env bash
# run-swarm.sh — a subnet participant in the correlation swarm. Emits several
# sessions carrying a SHARED fingerprint from THIS container's real subnet, so
# the engine's cross-session correlation sees one identity across three subnets.
# LOCAL TARGET ONLY: this container is on an internal lab network.
set -uo pipefail

BASE="${HM_BASE:-https://engine:8443}"
FP="${HM_FP:-swarm-shared-fp-0001}"
N="${HM_SWARM_N:-5}"

echo "[swarm] $(hostname) @ $(hostname -i 2>/dev/null) waiting for ${BASE} ..."
for i in $(seq 1 60); do
  if node -e "const https=require('https');https.get(process.argv[1]+'/api/session',{rejectUnauthorized:false},r=>{r.resume();process.exit(0)}).on('error',()=>process.exit(1))" "$BASE" 2>/dev/null; then break; fi
  sleep 2
done

echo "[swarm] emitting ${N} sessions with shared fp=${FP}"
node /app/swarm-collect.mjs "$BASE" "$FP" "$N"
