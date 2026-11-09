#!/bin/sh
set -eu
test -f /opt/lab/red-blue-product-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
