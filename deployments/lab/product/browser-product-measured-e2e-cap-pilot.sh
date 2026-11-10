#!/bin/sh
set -eu
# Cap0 browser-product measured e2e lander pilot (in-image). Contract + scaffold ≠ product green.
test -f /opt/lab/browser-product-measured-e2e-cap0-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
