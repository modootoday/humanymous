---
id: plan:root/20261106222418
title: Rust wargame engine simulation
status: active
created: 2026-11-06T22:24:18Z
updated: 2026-11-06T22:24:18Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
---

# Rust wargame engine simulation

## Goal

Provide a private, owned-lab-only Rust observation simulation using the existing
verdict engine, with honest unavailable-runtime and human-control accounting.

## Steps

- [x] Claim red-catalog/rust-wargame; inspect test taxonomy and sole Rust engine.
- [x] Write normative scope, serial persona synthesis and adversarial blockers.
- [x] Implement non-publishable attacker/defender contexts and seeded CLI ledger.
- [x] Test actual engine results, errors, missing coverage and human/ceiling split.
- [x] Run unit tests, Clippy and a network-none Docker engine pilot twice.
- [x] Independently verify ledger accounting and identical-seed bytes.
- [ ] Root owner integrates workspace members and lockfile, then validates actual workspace.
- [ ] Full Rust Gate/browser/challenge runtime, signed installation binding and
  internal-topology escape negative tests. These block network-runner enablement.

## Record

2026-11-07 Asia/Seoul: isolated snapshot passed 8 unit tests and Clippy with
warnings denied. Docker build uses Rust 1.88; both identical-seed executions
produced 12 scheduled cases: 8 evaluated, 4 unsupported, zero errors/skips or
regression mismatches. Evaluated outcomes: ALLOW 2, CHALLENGE 3, DENY 3. The
human baseline ALLOW and T4 ceiling ALLOW are distinct; long-window zero-motor
accessibility is existing HR-12 CHALLENGE. `human_controls_all_allow` is false.
This is a disclosed existing heuristic limitation, not permission to alter it.

The Docker CLI and credential helper were outside PATH; a process-local PATH
addition enabled real build/run. Container inspection confirms network none,
unprivileged user, read-only rootfs, dropped capabilities, no published ports or
mounts. Separate same-image IPv4/IPv6 documentation-address attempts failed with
loopback only and no default route. This does not prove future Gate topology.

Private raw evidence bundle: `.spec.local/root/evidence/20261106222914-rust-wargame/`.
Root integration instructions
are `crates/attacker/INTEGRATION.md`. No common Cargo files, Go, scoring, public
docs, runtime deployments, workflows or migration scripts were modified.
