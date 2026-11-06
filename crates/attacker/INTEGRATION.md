# Root integration request

Add `crates/attacker` and `crates/defender` to workspace members and regenerate
Cargo.lock under the root lane. Both manifests inherit workspace metadata/lints
and use existing serde/signals dependencies. No new external dependency is needed.
Run `cargo test -p humanymous-attacker -p humanymous-defender` and matching Clippy.
Until integrated, tests use a private temporary workspace snapshot.

`cargo run -p humanymous-attacker -- --seed 42` prints the actual engine report.
`test/rust-wargame/compose.yaml` is a local-only network-none engine pilot.
These packages must stay excluded from public runtime images/npm installations;
both are non-publishable. Future Gate runtime adapter must satisfy signed
installation binding and negative egress tests in sot:root/rust-wargame before
enabling network execution. This slice exposes no endpoint or target option.
