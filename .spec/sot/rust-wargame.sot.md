---
id: sot:root/rust-wargame
title: Rust wargame
status: active
created: 2026-11-06T22:24:18Z
updated: 2026-11-06T22:24:18Z
verify:
  - cargo test -p humanymous-attacker -p humanymous-defender
rel:
  governs: [crates/attacker, crates/defender, test/rust-wargame]
  depends_on: [sot:root/rust-migration]
---

# Rust wargame

## Rule

This slice is a deterministic **pure-engine simulation**, not runtime e2e.
The attacker crate owns an extensible, versioned catalog of synthetic observations
and replays the existing frozen Go-oracle fixtures. The defender invokes the
existing Rust `humanymous_scoring::engine::Engine::score_with_trace` unchanged.
Neither crate implements detection policy or alters weights, thresholds or rules.
Registry constructors supply scores for synthetic observations. Frozen reports
are read directly from scoring's canonical fixtures, never copied or regenerated.

Each scheduled case has a stable ID, tier T0–T4/control, seed, provenance, input,
expected regression outcome and exactly one terminal status: evaluated, error,
skipped or unsupported. A seed deterministically orders the catalog; it does not
claim stochastic coverage. The report includes actual score and complete trace,
input reports, attempted/evaluated/error/skip/unsupported denominators, and counts
ALLOW, CHALLENGE and DENY separately by class. Errors never become ALLOW or a
successful block. Regression mismatches fail the CLI; known unsupported runtime
cases remain visible and make runtime coverage incomplete. No percentage may
drop errors or unavailable cases from the scheduled denominator.

Human controls count explicit ALLOW separately from CHALLENGE and DENY. The
zero-motor baseline and its extended observation window exercise accessibility
regression risks without claiming measured human false-positive rates. A T4
coherent-observation replay is a ceiling control, never a detected bot success.
Pass token issuance/verification, actual accessibility/browser behavior, replay
storage, HTTP enforcement and observed network fingerprints require runtime
integration and must remain unsupported here.

### Isolation and installation binding

The CLI has no target/URL option and neither crate opens sockets. The engine pilot
runs as an unprivileged read-only Docker container with `network_mode: none`, no
published ports, no Docker socket and no host mounts. Build dependency downloads
are outside the execution sandbox. Both packages are `publish = false`; attacker
sources and binaries must never enter normal installations, npm packages or
public defender runtime images. The local pilot image is explicitly lab-only.

A future runtime runner MUST use an internal Docker network with no gateway to
the internet and only fixed service names. Operator-provided arbitrary URLs,
redirects, DNS rebinding and externally published ports are forbidden. Before
any probe it MUST authenticate the target's explicit installation identity using
a per-installation pinned public key and a fresh signed nonce. A matching string,
private IP or container hostname alone is insufficient authorization. Identity
failure, missing identity, route escape, redirect or egress-test failure aborts
before attacks. The runtime runner is not implemented in this slice; the CLI
rejects runtime mode. No unauthenticated pseudo-binding API is supplied.

### Design review and residuals

Blue: reuse the sole verdict engine. Red: synthetic inputs do not prove collection
or enforcement; expose unsupported cases. SRE: network-none pilot cannot prove
the future internal topology or installation binding; require signed-nonce and
egress negative pilots before runtime enablement. Privacy: only frozen/synthetic
reports, no live identifiers. Evaluator: distinguish challenge from human allow,
retain missing coverage and T4. Adversarial critique: runtime identity and escape
tests are P0 blockers for enabling any network runner; therefore no network
runner ships. No motor speed/richness requirement is added.

## Why

The migration requires replayable red/blue evidence while full Rust Gate is not
available. Success-only mock numbers would hide missing runtime responsibilities.

## Verification

Unit tests exercise actual engine replay, deterministic seeds, accounting,
negative CLI options, and human/ceiling distinctions. Run Cargo tests and Clippy
in a temporary isolated workspace until the root owner integrates membership.
Run the lab-only Docker engine pilot with no network and retain raw JSON privately.
Full runtime e2e remains subject to the existing Docker-only authority.

Docker's [none driver](https://docs.docker.com/engine/network/drivers/none/)
creates only loopback; the [Compose service contract](https://docs.docker.com/reference/compose-file/services/#network_mode)
provides `network_mode: none`. A normal [bridge](https://docs.docker.com/engine/network/drivers/bridge/)
provides external access by default, so a bridge alone is never isolation evidence.
These official references were checked on 2026-11-07 (Asia/Seoul).
