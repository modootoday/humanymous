---
id: evidence:root/20261107052501
title: Rust nginx native source-bound compatibility results
status: active
created: 2026-11-07T05:25:01Z
updated: 2026-11-07T22:11:53Z
files:
  - {path: native-results.json, sha256: 17aceea2fa59f73972f9fe6d6d431fa47bbd83dead978caa17197cdb063b570e, bytes: 9567, store: repo}
  - {path: git-byte-proof.json, sha256: 811e7f6bceb631d781e6085c2ebc88a97f639561490af0fd9dda9494287b099e, bytes: 8416, store: repo}
  - {path: source-only-checkpoint.json, sha256: 797c4a27e9ae32dda5c7d362e6470c0a7180d5b6f5fc5896baaf045d380fc390, bytes: 2027, store: repo}
  - {path: component-package-source-checkpoint.json, sha256: c8974c05b073b7557321c2aa9a30d5efd33996dad7c1ff3d16551df2a809fcb5, bytes: 2873, store: repo}
  - {path: latest-pure-verification.json, sha256: 0287357dd301e028cf59ca68db01ef0bef87a460c864ebd807f49b6c0400269b, bytes: 20224, store: repo}
  - {path: hosted-813-execution-summary.json, sha256: c0623cddf19f83ee2d0db992a83763726e7078431f3f657c4cd50398252b0d74, bytes: 1417, store: repo}
  - {path: hosted-813-source-proof.json, sha256: fd941d43e3ce5eb9b5bc8ce5f32dede14bbfdd7e4896d425c60c3a112be9a8d7, bytes: 7653, store: repo}
  - {path: eight-public-package-handoff.json, sha256: 305217206e59cadd4aecb15b4a2c78b895117abd4172dab59b9eb740189c362f, bytes: 12028, store: repo}
  - {path: core-slice01-consumer-imports.json, sha256: 911bbcdc63e46ed123fc8ef910a6a2b5c2d157816af4a3281ab8a2896fec9591, bytes: 3366, store: repo}
  - {path: runtime-slice02-consumer-imports.json, sha256: 2a3c3dcb48bdc2daef1fddd983065b75801c5c544fc9e4c0835da1407b3b83cf, bytes: 2131, store: repo}
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

component-package-source-checkpoint.json commits exact Git blob bytes at7cda20c
for the strict report, unsigned package and JSON/SSE cancellation follow-ups.
All runtime/build/unit/Clippy gates for that source remain NOT_RUN. No success
is inherited from16c18f3 or the unrelated live hosted run targeting63f69ae.

latest-pure-verification.json records the later actual Windows pure14 result,
package-feature binary check and scoped library/package Clippy success. Captured
nginx/direct dependency sources, root manifests/lock/toolchain and contract did
not change during those commands. This capture is not a complete transitive
source closure. Raw logs and before/after manifests remain in the private bundle.
Linux native/pilot Clippy, actual load/JSON/SSE/authority and full product gates
remain NOT_RUN for this source; the local pure result does not qualify them.

hosted-813-execution-summary.json is the recovered actual CI public summary from
source8132117/run42355168729/job128358355202:35/35 scenario groups, zero failed/
ignored/not_run, cleanup0, unchanged source and image identity. The accompanying
hosted-813-source-proof.json commits canonical Git blob hashes and separates the
synthetic ALLOW fixture from actual ApplicationHttp/executable/private-pipe/WAL
assertions. Successful pinned image construction enforces native/pilot/package
build and Clippy gates. Full raw per-case/build-stage logs and the package manifest
hash were not exported in the free runner's sanitized summary; do not invent them.

This hosted result supersedes the historical source-only NOT_RUN status only for
exact8132117. The a26d startup failure is retained separately with0jobs/0tests.
Full product remains unqualified, and sealed ServiceHost, future profiles and
uncommitted worktree changes inherit no success from this component receipt.
