---
id: plan:root/20261107202316
title: Delivery core consumer and Header scope integration handoff
status: active
created: 2026-11-07T20:23:16Z
updated: 2026-11-07T20:23:16Z
rel:
  depends_on: [sot:root/delivery, sot:root/structural-development, sot:root/rust-wargame]
---

# Mission and non-negotiables

Root accepted the eight-public-package baseline and assigned only the defender
CORE-SLICE-01 consumer imports to Delivery. Root owns every Cargo manifest/lock,
all directory relocation, publication decisions and reference synchronization.
The separate Header callsite correction stays inside existing Delivery paths.
Detection/scoring behavior, visibility and all unsafe/authority boundaries are
unchanged. No new crate, compatibility shim, directory move or import migration
outside the exact assigned consumer is performed here.

# Done and evidence

- Hosted Go run42406688473 at a19 completed successfully: eight cases and40
  checks, fresh Unix compile, Delivery110 and both job annotations0. Original
  remote private ledger and archive bytes remain unrecovered. Evidence is
  [[evidence:root/20261107194718]] and independent Delivery consumption in
  [[evidence:root/20261107052001]]; no promotion to a new Core source.
- Defender has only mechanical signals/scoring/integrity references changed to
  matching humanymous_core modules in two Rust files. Literal reverse mapping
  reproduced originals before formatting. No manifest or behavior change by
  Delivery. Root was explicitly authorized to apply only rustfmt to those two
  delegated files before its source freeze; the client hash below reflects that
  observed Root-side formatting.
- Header source correction now selects fixed private Semantic/ClosedProof
  descriptors from the existing CLI callsites. Three new memory-only tests
  cover dispatch, owner/source/proof failures without payload output, semantic
  no-frame behavior and closed writer/flush failures. Source is formatted;
  initial source checkpoint was uncompiled. The subsequent Root-allocated
  local window at ba588e22c30b1434044ec412716a976dd4ffcf8a executed all113
  tests successfully, plus all-target check/strict Clippy/fmt/policy with
  exit0. Its2112-file local snapshot had no differences. The sanitized
  receipt is attached under [[evidence:root/20261107052001]]. This is local
  Delivery evidence, not an actual closed producer or retained receiver.

# Exact owned source handoff

| Path | Current SHA256 | State |
| --- | --- | --- |
| crates/defender/src/lib.rs | 78f5f341c4a3cf1990ba4f70692100232285c960552eb13101860c47d789cafb | Root-delegated consumer import; no further writer edits |
| crates/defender/src/lab/client.rs | 3abae8a330ef86ca8b548172d0970483a767faa1ecfc81694dfbaa03a5d5f198 | Root-side rustfmt observed after initial99c1d6 handoff |
| scripts/ci/delivery/src/components.rs | f96dd028b75afe3535589654b71e126e4989df569f2719ecf78334814218d9d3 | Header scope split; subsequent local113 passed |
| [[sot:root/delivery]] | 3a35ad50b7b61e9a94a6f560887f91ea90139f2152e71650a6be2d1edbbd4600 | Header scope canon delta |
| [[plan:root/20261107172649]] | 1d14a78c156a2decc68422c6be25388a8c68b7860bcb46856ca4ff26ffbf68db | Exact9586 failure and new-source limits |

Root committed the first two delegated paths in atomic Core source checkpoint
c67d07c0114bd04793410f82f7182de7bfa6df2c, including its final rustfmt.
They need no second source commit. Core162, strict Clippy and collector WASM
checks do not qualify the defender consumer or the full workspace.
Header code/spec paths are a separate source-only checkpoint; they must not be
misreported as tested by c858/a19 or by the immutable9586 run. Root must rehash
before staging because other coordinated authors may advance common state.

# Active work and leases

Delivery remains sole monitor of existing full CI run42407744891,
source9586f7e488e4a0570d4fa7a1f89dfb4c2c05a56b, attempt1. Its persistent
gh-watch tool session is32133. Evidence preparation is private directory
.spec.local/root/evidence/20261107195555-delivery-full-ci-9586-monitor.
No local compiler or target lease is owned by Delivery. Do not terminate foreign
Cargo processes or restart the hosted run on an observation timeout.

The e2e-infra board claim remains owned by codex/rust-delivery-followup, expanded
with the disjoint Root-approved defender/src and defender/tests loan. This is
an integration handoff, not a provider/session end: the live watch and remaining
Delivery paths still require that claim. Release only the borrowed source
scope when Root confirms atomic integration; release the full lane when work is
actually handed over, not while its watcher is active.

# Remaining current-source gates

The full-native component header call requests semantic C11/C++11 conformance.
The standalone native-component header requests the unchanged R3 closed producer.
Neither a semantic pass nor a prepared producer buffer supplies actual retained
receiver completion. The Windows private-parent refusal remains unresolved;
Root must keep the distinct migration/shipping retention hold and decide its
concrete readiness integration. No grammar/profile relaxation or ambient job
name selects the mode.

Actual9586 Ubuntu owner generation/conformance passed; closed compiler-profile
observation failed. The later native workspace failure is separate:1170 passed,
zero failed tests, one accounting error,112 executed harnesses and incomplete
coverage. These old-source outcomes are not rewritten by the current fix.
The overall run remains monitored through terminal and all required statuses.

# Next actions

1. Root owns the committed Core Cargo/lock integration and the remaining
   consumer qualification. No consumer compile before explicit allocation.
2. Header source checkpoint d540269a and its allocated jobs1/incremental0
   local unit/check/strict-Clippy window are complete, without overlays or
   retry. No additional compiler lease is held.
3. Keep the original hosted watch and capture terminal jobs, annotations,
   source-specific failures and raw-evidence availability limits.
4. Route any adapter/native owner path changes through Root. Audit f9 source
   reconstruction remains fixed historical Git bytes and declared seams;
   never substitute current consolidated package sources.

# Commands to verify after Root authorizes the coherent window

- cargo test --locked -p humanymous-defender --all-targets
- cargo clippy --locked -p humanymous-defender --all-targets -- -D warnings
- cargo test --locked -p humanymous-delivery -- --list
- cargo test --locked -p humanymous-delivery
- cargo check --locked -p humanymous-delivery --all-targets
- cargo clippy --locked -p humanymous-delivery --all-targets -- -D warnings
- cargo run --locked -p humanymous-delivery -- policy

Use the final accepted package selectors if Root changes a package role in that
same atomic slice. No --all-features substitute or new actual process/SCM/ACL
pilot follows from these pure commands. Actual e2e remains Docker-only and needs
its separately allocated hosted profile.

# Files to read

Read AGENTS.md, .agents/lessons/HARD-WON.md, the session board and canonical
[[sot:root/structural-development]] and [[sot:root/delivery]]. Header details:
[[plan:root/20261107172649]]. Raw source-bound observations stay in private
Specfold evidence, with sanitized public projections under Delivery evidence.
Provider residual chat state remains in Codex; no Claude memory is required.

# Out of scope and residual risks

No path/package rename beyond the assigned imports, no mass manifest edit,
publisher/OIDC exchange, deployment, tag, main merge, unallocated compiler,
new hosted trigger, erasure or repair. Same-SID/privileged-OS ceilings and
unknown cleanup remain explicit. Source-only syntax/format checks are not
compiled feature/artifact, native-provider, browser-wire or full-product proof.
