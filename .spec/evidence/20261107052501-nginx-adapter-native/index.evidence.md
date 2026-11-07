---
id: evidence:root/20261107052501
title: Rust nginx native source-bound compatibility results
status: active
created: 2026-11-07T05:25:01Z
updated: 2026-11-07T06:04:29Z
files:
  - {path: native-results.json, sha256: 17aceea2fa59f73972f9fe6d6d431fa47bbd83dead978caa17197cdb063b570e, bytes: 9567, store: repo}
---

# Rust native nginx compatibility

Only sanitized summaries/hashes are attached here. Private commands, immutable
dependency source snapshots and failed/retried logs are retained in
.spec.local/root/evidence/20261107052500-nginx-adapter-native.
Working validated snapshots and canonical Git commits are distinct.
native-results.json records the validated working snapshot; git-byte-proof.json
will bind that owned source to its canonical commit without rewriting old results.

This is nginx/authority component qualification, not full Gate/collector/Pass/
browser/human/AT or production qualification. Positive ALLOW uses an explicit
trusted synthetic provider through actual shared authority and durable WAL;
actual production ApplicationHttp outcomes are reported separately.
