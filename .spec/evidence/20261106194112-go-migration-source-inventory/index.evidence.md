---
id: evidence:root/20261106194112
title: Go migration source inventory
status: active
created: 2026-11-06T19:41:12Z
updated: 2026-11-06T22:57:20Z
files:
  - {path: go-inventory.json, sha256: 2f71baf6befd97b385e1cbd092f3fc4d78b1f39205c4d51ce6a655bfe290b0bb, bytes: 115948, store: repo}
---

# Go migration source inventory

## What was measured

Every Go file tracked under internal/, cmd/ and pkg/ at baseline
981d25cf71a5a54855d7e2b4359e832a9d5998e5. The attached JSON records file
paths, SHA-256 digests, Test-function names and pending Rust mappings.

## How to read it

Each row starts pending. A source file is migrated only when all its
responsibilities and tests have Rust mappings plus parity evidence. Pending rows
are not an implementation claim. The inventory includes browser build-tagged
sources that host `go test` cannot exercise. Hashes detect stale oracle inputs.
