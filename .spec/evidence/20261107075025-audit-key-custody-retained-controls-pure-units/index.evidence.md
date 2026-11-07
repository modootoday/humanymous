---
id: evidence:root/20261107075025
title: Audit key custody retained controls pure units
status: active
created: 2026-11-07T07:50:25Z
updated: 2026-11-07T07:58:13Z
rel:
  governs: [crates/audit/src/control]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: a7c28a57a1778c0deadd63eb3f2fa44ec8971f685d156f19d0c6543aacbd184b, bytes: 2073, store: repo}
---

# Audit key custody retained controls pure units

## What was measured

Authored source for independent signed control-anchor retention, verified private
pipe client/service, explicit durability and retained-only action capabilities.
Seven deterministic in-memory tests are authored; no Cargo execution has occurred
yet. Formatting and scoped public Specfold checks are separate source checks.

Seven actual-filesystem integration tests are authored in control_operations:
independent restore/rollback refusal, retention failure with durable nonce spend,
exact authenticated pending recovery, sealed-owner dual lifetime locks with
missing-checkpoint repair and complete ambiguous-tail preservation without
checkpoint acknowledgement. These require hosted execution; no new ignored
helpers or helper-accounting exceptions are introduced.

Prepared rotation now commits both source and encrypted candidate before approval
and uses the retained-only action grant when applying exact candidate bytes.
Its pure target-binding unit is the seventh pure test. Two additional hosted
tests cover unchanged identity/vault/witness material and wrong/empty credential
refusal, substituted destination and stale source refusal without extra writes.
Initial source-before manifest predates that added source; neither snapshot has
an executed result yet. Do not combine snapshots with later logs without exact
hash matching.

## How to read it

Compilation, pure test results, actual files/ACL/locks/crash/process/IPC,
independent OS owner/volume, runtime mutation and product Docker qualification
are currently not_run. Authored test counts are not passed-test counts. No paid
resources, workflow dispatch, old-source green reuse or publication is claimed.
