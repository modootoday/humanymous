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

Two Rust tests authored: independently mutate each required input and verify
the recorded digest AND complete map change, and require the actual
verify_snapshot to reject the old snapshot for each mutation (absolute fixture
keys, no process cwd change), then accept restored bytes; nested unrelated manifest
included and build/private fixture manifests excluded; independently remove each
required input and require inventory refusal. They use existing fixture custody.
Tests are NOT_RUN pending Root's compiler/test allocation. Bare rustfmt and
whitespace checks are source-only; no native actor/runtime/SCM grant is implied.
Root must requalify source inventory and whole moved consumer graph separately.

Recommended bounded selector after allocation:
cargo test --locked --manifest-path scripts/ci/windows-native-runner/Cargo.toml
evidence::source_inputs::tests, followed by scoped all-target check/strict Clippy.
No fixture process launch is needed by these two directory/digest tests.
The platform two-context mapping research remains pending after this correction.
