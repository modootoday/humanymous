---
id: evidence:root/20261107103417
title: Apache adapter hosted ABI compile failure
status: active
created: 2026-11-07T10:34:17Z
updated: 2026-11-07T10:35:01Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 49ebd865eeeac7c547798f46712a3925af3ccff127551061ce5a3d7516a98880, bytes: 1631, store: repo}
---

# Apache adapter hosted ABI compile failure

## What was measured

Actual standard GitHub Ubuntu24.04 run42363960400, component job128377977909,
selected source245abeda95d269bb14f22e2980cf494a40e02961. Checkout and Rust1.99.1
setup passed. Safe units passed9and coordinator units passed10, each with zero
failures, ignored or filtered tests. Linux pilot all-target check and both safe
and pilot-feature warnings-denied Clippy passed.

The actual Docker ABI build exited1 after259.452seconds. Its safe diagnostic
projected E0282 at crates/apache-adapter/native/src/host.rs:35:40, the ambiguous
AP_MPMQ_RUNNING as _ comparison. This selected-httpd code was excluded from the
earlier pilot-only checks. The fix explicitly uses c_int for MPM query inputs,
output state/generation and state comparison without changing the predicate.

The coordinator verified the actual hosted Linux Docker endpoint/server identity
and reported owned container/image cleanup. The separate always-cleanup step
also passed with its own success receipt. No private bundle was uploaded.
Full job logs were captured privately by the owner and root.

## How to read it

No native image or Apache scenario completed. Image/artifact identities and
observed counts remain null; all45required scenarios are NOT_RUN and
fullProductSuccess is false. The historical failure receipt's source fields
were null even though its outer actual run SHA was known. A source-only fix
preserves verified archive source/manifest identity through future failures;
unknown image, artifact and case identities remain unknown. No later result
is inherited by these exact failed bytes. Actual ABI retry is the required gate.
