---
id: evidence:root/20261107084330
title: Pending-handshake pure and hosted-feature compile qualification
status: active
created: 2026-11-07T08:43:30Z
updated: 2026-11-07T08:43:30Z
files:
  - {path: receipt.json, sha256: 0d81becec19e1597432d8a6932133010095905f03d44b0cf2d03a549aeebef9a, bytes: 14682, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107072531]
---

# Local B qualification for pending-handshake source

Root explicitly assigned target/validation-agents, buildjobs1/incremental0.
Source checkpointb0363e9 remained unchanged. Recursive capture/network/signals
files plus root Cargo.toml/Cargo.lock/rust-toolchain.toml were recorded before
and after:60files,0changes, identical snapshot SHA256. Registry dependencies
remain bound by the unchanged locked graph; registry source trees were not
individually snapshotted. This is selected-package qualification, not a complete
workspace pre/post receipt or a qualification of hosted813's other WIP.
Only validation prose/evidence is updated after the gate; compiled Rust and
manifests stay unchanged. The60file snapshot identifies the tested point, not
an assertion that later documentation bytes are identical.

| Command/gate | Actual result |
|---|---|
| Locked selected lib tests |22passed,0failed,0ignored,0filtered,0measured;8.930s |
| Locked all-targets Clippy with hosted-pilot, warnings denied | Exit0;5.046s |
| Locked hosted_transport build with hosted-pilot and --no-run | Exit0;2.720s; executable compiled, never run |

The22units include compile-bound PendingHandshake/Connection/Cancellation Send
and Cancellation Sync. Actual pre/mid/post-handshake behavior remains the five
standard-hosted pilots' responsibility. No TLS/H2/socket test ran locally, no
Docker was started and no binary was published/uploaded. Executable hashes,
compiler identity, exact commands,60source hashes, raw evidence hashes and the
stable lock context are in the attachment.

The first attempt failed before testing: locked offline full-workspace metadata
could not download cached-missing asn1-rs0.7.2. Its error and unchanged context
are preserved. Root then approved public dependency downloads and reassigned B
for selected-package locked-online commands, without regenerating the lock or
retrying broad metadata. All three selected gates passed and B was returned
immediately. The first failure is not relabeled as a test failure.

Open gates: actual5hosted cases, complete runtime body/stream/worker custody,
Docker product qualification and original-hop topology. Failed813Linux/Docker
job receipts remain independent and unchanged.
