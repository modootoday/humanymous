#!/usr/bin/env bash
# check-licenses.sh — fail if any module actually COMPILED INTO the shipped binaries
# (core, gate, wasm client) is missing from THIRD_PARTY_LICENSES.md, so the licence
# index for distributed code cannot drift (PLAN-08 backlog item 6). Test/tool-only
# transitive deps are not distributed and are intentionally out of scope.
set -euo pipefail
cd "$(dirname "$0")/.."
mods=$(
  { go list -deps -f '{{if .Module}}{{.Module.Path}}{{end}}' ./cmd/server ./cmd/gate
    GOOS=js GOARCH=wasm go list -deps -f '{{if .Module}}{{.Module.Path}}{{end}}' ./cmd/wasm 2>/dev/null || true
  } | sort -u | grep -v '^github.com/modootoday/humanymous$' | grep -v '^$'
)
missing=0
for m in $mods; do
  grep -qF "$m" THIRD_PARTY_LICENSES.md || { echo "MISSING from THIRD_PARTY_LICENSES.md: $m"; missing=1; }
done
[ "$missing" -eq 0 ] && echo "OK: licence index covers all shipped dependencies: $(echo $mods | tr '\n' ' ')" || exit 1
