#!/bin/sh
set -eu
test -f /opt/lab/g6-origin-audit-binding-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
