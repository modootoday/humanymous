#!/bin/sh
set -eu

exec node scripts/external-input-smoke.mjs "$@"
