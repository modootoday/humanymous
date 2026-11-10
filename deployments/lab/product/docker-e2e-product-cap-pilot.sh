#!/bin/sh
set -eu
# Cap0 Docker e2e lander pilot (in-image). Presence of contract + scaffold receipt
# is Cap0 honesty only; never invent docker_e2e_validate_product / pass_product.
test -f /opt/lab/docker-e2e-product-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
