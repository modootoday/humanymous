---
id: evidence:root/20261107075756
title: Rust wargame shared decoder acceptance
status: active
created: 2026-11-07T07:57:56Z
updated: 2026-11-07T08:01:26Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: summary.json, sha256: 164b1b908059bae5e0133da49c4666683bb8d1fec4664baab890347854d168a2, bytes: 808, store: repo}
---

# Rust wargame shared decoder acceptance

## What was measured

Windows x86_64 Rust1.99.1, root-assigned A shared-target slot:28pure checks passed
(attacker15+CLI1+verifierCLI1+defender9+shared JSON2),0failed/0ignored.
Shared JSON ran with --lib json:: and9unrelated tests filtered out; those tests
were not executed. All-target scoped Clippy for attacker/defender/test-support
passed with -Dwarnings. Both attempts passed; they are not added into56checks.
Attempt2 followed correction of attempt1's incomplete source inventory, which had
omitted behavior/model. No application process, socket, IPC or Docker test ran.

The consumer source is052c1da. Attempt2 preserved actual worktree bytes for13
local crates plus root Cargo.toml/Cargo.lock/toolchain/config paths, then compared
snapshot hashes against before hashes and captured the same path list afterward.
Before HEADa7be85e39ab15c8f6606ff5c3f30ff7536571fb8; after
0b51b3f12694d849c94179149315b47ef00d0d44. Three source hashes changed:
audit keys/custody/rotation.rs gained a cfg(test) module; audit's independent
control_operations integration test changed; runtime INTEGRATION.md changed.
These are outside the selected executed tests and the normal dependency build's
enabled test configuration. The whole-tree fingerprint still moved; do not call
it an immutable clean graph. Consumer, shared decoder and root lock bytes matched
before/after. Raw snapshot, hashes, toolchain and both attempts remain private in
evidence:root/20261107075313.

The common decoder was subsequently committed as f44b32b. Its committed blob,
current raw worktree blob and attempt2's captured blob all match
9a9a57a53225c54e944ccf8ca7488a58c8609270. This binds that decoder's source
checkpoint to the measured tests without treating later fixture APIs as tested.

## How to read it

This is actual local shared-decoder regression evidence, not a reuse of v0.5's
26checks or proof of hosted network execution. Cargo jobs1/incremental0 and test
threads1 were retained; A was returned immediately after checks and source capture.
The initial archive preparation failed before any Cargo command and was corrected;
no failed test was erased. Fresh immutable hosted strict-byte generation and its
safe receipt remain pending. Unsupported7 and full-product=false stay explicit.
