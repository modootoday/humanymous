---
id: evidence:root/20261107052501
title: Rust nginx native source-bound compatibility results
status: active
created: 2026-11-07T05:25:01Z
updated: 2026-11-07T06:23:28Z
files:
  - {path: native-results.json, sha256: 17aceea2fa59f73972f9fe6d6d431fa47bbd83dead978caa17197cdb063b570e, bytes: 9567, store: repo}
  - {path: git-byte-proof.json, sha256: 811e7f6bceb631d781e6085c2ebc88a97f639561490af0fd9dda9494287b099e, bytes: 8416, store: repo}
  - {path: source-only-checkpoint.json, sha256: 797c4a27e9ae32dda5c7d362e6470c0a7180d5b6f5fc5896baaf045d380fc390, bytes: 2027, store: repo}
---

# Rust native nginx compatibility

Only sanitized summaries/hashes are attached here. Private commands, immutable
dependency source snapshots and failed/retried logs are retained in
.spec.local/root/evidence/20261107052500-nginx-adapter-native.
Working validated snapshots and canonical Git commits are distinct.
native-results.json records the validated working snapshot; git-byte-proof.json
binds that owned source to canonical16c18f3 with identical working/Git SHA256 and
LF-only bytes. Shared dependency snapshots and later commits remain separate.

This is nginx/authority component qualification, not full Gate/collector/Pass/
browser/human/AT or production qualification. Positive ALLOW uses an explicit
trusted synthetic provider through actual shared authority and durable WAL;
actual production ApplicationHttp outcomes are reported separately.

source-only-checkpoint.json records the later P1 JSON-response source checkpoint.
Its native build/Clippy/JSON/runtime gates are NOT_RUN under root's memory-driven
single-heavy-slot scheduling. The old35-group result must not qualify this new
callback path. Formatting/document checks are not native execution.
