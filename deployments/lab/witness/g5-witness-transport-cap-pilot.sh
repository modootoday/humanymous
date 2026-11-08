#!/bin/sh
set -eu
test -f /opt/lab/g5-witness-transport-contract.json
test -f /opt/lab/measure-receipt.json
cat /opt/lab/measure-receipt.json
