#!/bin/sh
set -eu

out=/pki
umask 077
mkdir -p "$out"

if [ ! -s "$out/ca.pem" ] || [ ! -s "$out/core.pem" ] || [ ! -s "$out/core-key.pem" ]; then
  rm -f "$out/ca.pem" "$out/ca-key.pem" "$out/core.pem" "$out/core-key.pem" "$out/core.csr" "$out/core.ext"
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -sha256 -nodes \
    -subj "/CN=humanymous external-input lab CA" \
    -days 2 \
    -keyout "$out/ca-key.pem" \
    -out "$out/ca.pem"
  openssl req -newkey ec -pkeyopt ec_paramgen_curve:P-256 -sha256 -nodes \
    -subj "/CN=core" \
    -keyout "$out/core-key.pem" \
    -out "$out/core.csr"
  {
    echo "basicConstraints=critical,CA:FALSE"
    echo "keyUsage=critical,digitalSignature,keyEncipherment"
    echo "extendedKeyUsage=serverAuth"
    echo "subjectAltName=DNS:core,DNS:humanymous.local"
  } >"$out/core.ext"
  openssl x509 -req \
    -in "$out/core.csr" \
    -CA "$out/ca.pem" \
    -CAkey "$out/ca-key.pem" \
    -CAcreateserial \
    -days 2 \
    -sha256 \
    -extfile "$out/core.ext" \
    -out "$out/core.pem"
fi

chown 65532:65532 "$out/core.pem" "$out/core-key.pem"
chmod 0444 "$out/ca.pem" "$out/core.pem"
chmod 0400 "$out/core-key.pem"
openssl verify -CAfile "$out/ca.pem" -verify_hostname core "$out/core.pem"
touch "$out/ready"
