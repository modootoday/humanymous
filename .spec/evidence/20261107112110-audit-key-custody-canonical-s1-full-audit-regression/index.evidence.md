---
id: evidence:root/20261107112110
title: Audit key custody canonical S1 full audit regression
status: active
created: 2026-11-07T11:21:10Z
updated: 2026-11-07T11:23:19Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: efeb12e4155a59775b02bfc11bdb9ea19cd7156fad14d0317ef45b0698c0fb66, bytes: 4316, store: repo}
---

# Audit key custody canonical S1 full audit regression

## What was measured

Root renewed A for full audit regression at frozen sourcef88e76f4 after its
record-focused check/Clippy and9 tests passed. Ordinary-user Windows token,
jobs1/incremental0/testthreads1, exclusive target/validation-audit. No repeat
check/Clippy was needed:114-file closure matched the prior passing batch before
this run and remained unchanged afterward. Full locked all-targets test exit0.

Actual115 passes: library56, checkpoint parity2, controls21, witness13, backups10,
custody10, Merkle/record/verification parity3. Rawignored5 excluded, two zero-test
bins excluded, no failed/filtered/measured cases.11 harnesses have one outer
summary each; all exact helper identities and their actual required parents
reconcile, with no private-marker or nested-summary leakage. facts.json binds
per-harness results, original full log and source-manifest hashes. Original
commands/manifests/logs remain private in evidence111549. All handles terminal,
A returned. Audit Rust remains frozen through dependent Runtime C/.NET D checks.

## How to read it

The focused9 cases are a subset of the full115, not124 unique tests.92 unique
canonical vectors and13464 Unicode texts are fixture assertions, not additional
Rust test cases. This qualifies current partial-S1 canonical/NFC code against
the complete audit package on ordinary-user Windows; it does not complete S1,
measure128MiB memory/RSS, implement production streaming recovery, or qualify
hosted/protected profiles, strict native actor or product e2e. Legacy Vec-returning
and compatibility-mode APIs remain explicit. No SCM/accounts/network/Docker.
