#!/usr/bin/env bash
# scripts/e2e.sh — one-shot Red vs Blue run: build, start Blue, run Red catalog,
# generate the report, tear down. Local target only. Git Bash / Linux.
set -euo pipefail
cd "$(dirname "$0")/.."

ADDR="${ADDR:-:8443}"
RUNS="${HM_RUNS:-1}"

echo "[1/5] building wasm + server + report + redteam clients…"
GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/
go build -o bin/server.exe ./cmd/server/
go build -o bin/report.exe ./cmd/report/
go build -o bin/tlsparrot.exe ./cmd/tlsparrot/
go build -o bin/redteam.exe ./cmd/redteam/

echo "[2/5] stopping any stale server…"
{ taskkill //F //IM server.exe >/dev/null 2>&1 || true; }
{ command -v pkill >/dev/null && pkill -f bin/server.exe >/dev/null 2>&1 || true; }
sleep 1

echo "[3/5] starting Blue server on $ADDR…"
./bin/server.exe -addr "$ADDR" -web web >/tmp/hmserver.log 2>&1 &
SRV_PID=$!
sleep 3

cleanup() { taskkill //F //IM server.exe >/dev/null 2>&1 || kill "$SRV_PID" 2>/dev/null || true; }
trap cleanup EXIT

echo "[4/5] running Red catalog (HM_RUNS=$RUNS)…"
HM_RUNS="$RUNS" node test/e2e/runner.mjs || true

echo "[5/5] generating docs/report.html…"
./bin/report.exe -in test/e2e/results.json -out docs/report.html

echo "done. open docs/report.html"
