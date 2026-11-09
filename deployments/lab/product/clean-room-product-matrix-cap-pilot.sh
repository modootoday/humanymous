#!/bin/sh
set -eu
test -f /opt/lab/clean-room-product-matrix-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
