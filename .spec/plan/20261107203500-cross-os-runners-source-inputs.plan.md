---
id: plan:root/20261107203500
title: Native source inventory nested manifests and compiled inputs
status: draft
created: 2026-11-07T20:35:00Z
updated: 2026-11-07T20:35:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/delivery]
---

# Root-assigned source custody correction

Audit found source(false) collected the consolidated core src but omitted its
nested Cargo.toml: the old manifest scan visited only immediate crates children.
Both inventory modes also omitted production include_str JSON, because sources
accepts only rs/toml/lock. This is source-custody debt, not an observed runtime
semantic failure. Historical source receipts are never rewritten or qualified.

The shared source_inputs collector runs AFTER either existing inventory mode.
It recursively records Cargo.toml under crates, rejecting linked entries and
skipping target/.git/.spec.local subtrees. It separately REQUIRES the actual
core Cargo.toml and signals/wire/unicode.json plus signals/observation/registry.json
as regular files and hashes their bytes. Missing required input refuses the
snapshot; no extension omission becomes success. Other source, cleanliness,
Git-byte and evidence controls are unchanged. No manifest/dependency change.

Three Rust tests authored: independently mutate each required input and verify
the recorded digest AND complete map change, and require the actual
verify_snapshot's implementation to reject the old snapshot for each mutation
(explicit fixture root, no process cwd change), then accept restored bytes; nested unrelated manifest
included and build/private fixture manifests excluded; independently remove each
required input and require inventory refusal; independently omit each required
snapshot entry and require verifier refusal even when disk files still exist.
Both newly collected and supplied Options.source snapshots require all three
current keys; there is no permissive historical-snapshot fallback. Existing
historical receipts remain unchanged and unqualified. Tests use existing fixture custody.
Tests are NOT_RUN pending Root's compiler/test allocation. Bare rustfmt and
whitespace checks are source-only; no native actor/runtime/SCM grant is implied.
Root must requalify source inventory and whole moved consumer graph separately.

Recommended bounded selector after allocation:
cargo test --locked --manifest-path scripts/ci/windows-native-runner/Cargo.toml
evidence::source_inputs::tests, followed by scoped all-target check/strict Clippy.
No fixture process launch is needed by these three directory/digest tests.
The platform two-context mapping research remains pending after this correction.

## Allocated actual checkpoint (first failure preserved)

Root allocated new target/native-source-guard-v0, jobs1/incremental0, locked
offline, one monotonic390s work/450s cleanup pair. Actual tool session43027
terminated exit1 after34.96s. Inventory exactly3, actual3 passed/0 failed/
0 ignored/0 measured/55 filtered; runner all-target check and strict Clippy0.
Final fmt --all exited1 on dependency workspace authority/configuration.rs and
network/correlation/tests.rs formatting. No rerun or source edit during execution.

Observed HEAD1e8c5af6844915090840df834e262eec51cc9e3c plus uncommitted foreign
receipt input (SHAa24d1ca965d724589a3a5613af38747d1d4295448173fb65e1629f640ebb44df),
NOT a pure f059 Git candidate. Selected source/config/tool81 files and registry
2079 files were byte-identical before/after. Public native-source-guard-summary
registers34 raw attachments and actual test executable hash; scope is limited.

Original metadata retained lease_removed=false because typed DateTime/string
comparison refused cleanup. Separate cleanup addendum matched exact original
clock/provider/session/target via DateKind String, confirmed recorded PIDs absent,
then removed only the own marker at59.36s within ORIGINAL450. No renewed pair.
All owned processes terminal, no current lease; target is cache only. Existing
fixture Drop ran but per-directory absence was not independently instrumented.
No provider/account/token/SCM/Job/GUI/product qualification. Source guard's narrow
closure awaits independent audit; first fmt failure remains separate and open.
