#!/usr/bin/env bash
# ci-overlays.sh — stand up each PLAN-08 compose overlay and run its Docker consistency
# assert (PLAN-08 backlog item 7). Used by CI and reproducible locally.
set -euo pipefail
BASE="-f deployments/compose.yaml"
cd "$(dirname "$0")/.."

go run ./scripts/gen-demo-keys

# Start from a clean slate: the CI job leaves the base detection stack (core/origin/gate)
# up from the attack run, which shares this compose project + host port 8444 with every
# overlay below. Tearing it down first stops the first overlay's `up -d` from racing the
# still-running base gate for the port.
docker compose $BASE down -v >/dev/null 2>&1 || true

run() { # name  "services"  assert.mjs  [wait]  [env...]
  local name=$1 services=$2 assert=$3 wait=${4:-6}; shift 4 || shift $#
  echo "::group::overlay $name"
  local attempt ok=""
  # Retry the whole up→assert cycle: overlay containers occasionally lose the port-8444
  # race during the previous overlay's teardown on a loaded CI runner (the gate comes up
  # but its host mapping is not live, so the assert sees connection-refused for its full
  # readiness window). A retry from a clean teardown recovers a transient race; a real
  # breakage still fails all attempts. Gate logs are dumped on each miss for diagnosis.
  for attempt in 1 2 3; do
    docker compose $BASE -f "deployments/compose/$name.yaml" up -d $services
    sleep "$wait"
    if env "$@" node "scripts/$assert"; then ok=1; break; fi
    echo "overlay $name: attempt $attempt failed — gate logs follow, then teardown + retry"
    docker compose $BASE -f "deployments/compose/$name.yaml" logs --tail=25 gate 2>/dev/null || true
    docker compose $BASE -f "deployments/compose/$name.yaml" down -v >/dev/null 2>&1 || true
    sleep 4
  done
  docker compose $BASE -f "deployments/compose/$name.yaml" down -v >/dev/null 2>&1 || true
  echo "::endgroup::"
  [ -n "$ok" ] || { echo "overlay $name FAILED after 3 attempts"; return 1; }
}

run redis-ha       "origin redis gate gate-b" assert-shared-state.mjs
run proxyproto     "origin gate haproxy"      assert-proxyproto.mjs
run webbotauth     "origin gate"              assert-webbotauth.mjs
run privacypass    "origin gate"              assert-privacypass.mjs
run webauthn       "origin gate"              assert-webauthn.mjs
run redis-hardened "origin redis gate"        assert-redis-hardening.mjs
run audit-ch       "origin clickhouse gate"   assert-audit-clickhouse.mjs 16
echo "ALL PLAN-08 overlays passed"
