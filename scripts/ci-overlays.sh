#!/usr/bin/env bash
# ci-overlays.sh — stand up each PLAN-08 compose overlay and run its Docker consistency
# assert (PLAN-08 backlog item 7). Used by CI and reproducible locally.
set -euo pipefail
BASE="-f deployments/compose.yaml"
cd "$(dirname "$0")/.."

go run ./scripts/gen-demo-keys

run() { # name  "services"  assert.mjs  [wait]  [env...]
  local name=$1 services=$2 assert=$3 wait=${4:-6}; shift 4 || shift $#
  echo "::group::overlay $name"
  docker compose $BASE -f "deployments/compose/$name.yaml" up -d $services
  sleep "$wait"
  env "$@" node "scripts/$assert"
  docker compose $BASE -f "deployments/compose/$name.yaml" down -v
  echo "::endgroup::"
}

run redis-ha       "origin redis gate gate-b" assert-shared-state.mjs
run proxyproto     "origin gate haproxy"      assert-proxyproto.mjs
run webbotauth     "origin gate"              assert-webbotauth.mjs
run privacypass    "origin gate"              assert-privacypass.mjs
run webauthn       "origin gate"              assert-webauthn.mjs
run redis-hardened "origin redis gate"        assert-redis-hardening.mjs
run audit-ch       "origin clickhouse gate"   assert-audit-clickhouse.mjs 16
echo "ALL PLAN-08 overlays passed"
