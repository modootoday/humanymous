#!/bin/sh
# ci-overlays.sh — stand up each PLAN-08 compose overlay and run its Docker consistency
# assert (PLAN-08 backlog item 7). Used by CI and reproducible locally.
set -eu
PROJECT="${E2E_PROJECT_NAME:-hmn-overlays-${CI_RUN_ID:-$$}}"
PROJECT="$(printf '%s' "$PROJECT" | tr '[:upper:]_' '[:lower:]-' | tr -cd 'a-z0-9-')"
BASE="-p $PROJECT -f deployments/compose.yaml -f deployments/compose/assertions.yaml"
cd "$(dirname "$0")/.."

# Start from a clean slate: the CI job leaves the base detection stack (core/origin/gate)
# up from the attack run, which shares this compose project + host port 8444 with every
# overlay below. Tearing it down first stops the first overlay's `up -d` from racing the
# still-running base gate for the port.
docker compose $BASE down -v >/dev/null 2>&1 || true

run() { # name  "services"  assertion-service  [wait]
  local name=$1 services=$2 assertion=$3 wait=${4:-2}
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
    if docker compose $BASE -f "deployments/compose/$name.yaml" run --rm "$assertion"; then ok=1; break; fi
    echo "overlay $name: attempt $attempt failed — gate logs follow, then teardown + retry"
    docker compose $BASE -f "deployments/compose/$name.yaml" logs --tail=25 gate 2>/dev/null || true
    docker compose $BASE -f "deployments/compose/$name.yaml" down -v >/dev/null 2>&1 || true
    sleep 4
  done
  docker compose $BASE -f "deployments/compose/$name.yaml" down -v >/dev/null 2>&1 || true
  echo "::endgroup::"
  [ -n "$ok" ] || { echo "overlay $name FAILED after 3 attempts"; return 1; }
}

run redis-ha       "origin redis gate gate-b"              assert-shared-state
run proxyproto     "origin gate haproxy"                   assert-proxyproto
run webbotauth     "origin gate"                           assert-webbotauth
run privacypass    "origin demo-keys-init gate"            assert-privacypass
run webauthn       "origin demo-keys-init gate"            assert-webauthn
run redis-hardened "origin redis gate"                     assert-redis-hardening
run audit-ch       "origin clickhouse gate"                assert-audit-clickhouse 4
echo "ALL PLAN-08 overlays passed"
