#!/bin/sh
set -eu
test -f /opt/lab/g2-collector-product-binding-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
