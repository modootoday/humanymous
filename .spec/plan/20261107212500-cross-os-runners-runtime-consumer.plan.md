---
id: plan:root/20261107212500
title: Native runner consumer remap for consolidated Runtime contexts
status: draft
created: 2026-11-07T21:25:00Z
updated: 2026-11-07T21:25:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/structural-development]
---

# Root-authorized source-only handoff

Root moved six contexts to crates/sdk/humanymous-runtime; owns all Cargo/lock/
feature edits and new Runtime source. Native owner edits ONLY runner Rust consumers,
metadata role/source/selector mapping and their tests. No context source, manifest,
alias, unsafe expansion, detection/authority change or compiler run is authorized.

Native-security calls now use humanymous_runtime::native_security after applying
old Runtime-root-to-gateway replacement first. No old Runtime-root imports were
present to cascade into these new prefixes. Native build/test package lists are
runtime+CLI, replacing retired Authority/native-security package selections.
CLI service feature flags remain unchanged; Root-authored features select gateway/
native contexts. Retained binary names humanymous-runtime/humanymous-authority/
humanymous are executable names, not retired package aliases.

Source(false) collects one Runtime src and tests tree (including context fixtures),
current platform/core roots and CLI/runner, with existing recursive manifests and
exact three required core inputs unchanged. Runtime root replaces old native-security
own-Git-byte prefix. Existing metadata source binding now checks only current
platform/Runtime/CLI owning manifests; no five-retired-package mapping remains.

# Shared package does not collapse context roles

Opaque Cargo package IDs still join exact same-source workspace metadata with
member/local-source/version/manifest validation. Closed Role variants additionally
require exact target name, kind AND metadata src_path, with exactly one matching
target record; duplicate target associations refuse. Shared Runtime package ID
alone cannot grant another context. No name prefix wildcard or old-ID fallback.

| Role | Exact target | Kind | Source beneath its owning package |
| --- | --- | --- | --- |
| PlatformLibrary |humanymous_platform|lib|src/lib.rs (platform) |
| RuntimeLibrary |humanymous_runtime|lib|src/lib.rs (Runtime shared lib) |
| GatewayNative |gateway-native-process|test|tests/gateway/native_process.rs |
| AuthorityNative |authority-native-process|test|tests/authority/native_process.rs |
| AuditDurableWitness |audit-durable-witness|test|tests/audit/durable_witness.rs |
| AuditKeyBackups |audit-key-backups|test|tests/audit/key_backups.rs |
| AuditKeyCustody |audit-key-custody|test|tests/audit/key_custody.rs |
| CliService |service_process|test|tests/service_process.rs (CLI) |
| CliRuntimeService |runtime_service_process|test|tests/runtime_service_process.rs (CLI) |
| GatewayBinary |humanymous-runtime|bin|src/gateway/main.rs |
| AuthorityBinary |humanymous-authority|bin|src/authority/main.rs |
| CliBinary |humanymous|bin|src/main.rs (CLI) |

RuntimeLibrary means SHARED lib, not blanket Audit/Authority identity. Its only
declared captured helper is audit::storage::wal::tests::process_lock_probe and
exact parent audit::storage::wal::tests::exclusive_writer_rotation_checkpoint_and_restart.
Platform helper and six parent strings remain exact. Audit durable/key-backup
integration helper strings remain unchanged under renamed targets. Audit custody
recognition uses only AuditKeyCustody. Authority ignored privileged parent uses
only AuthorityNative, never GatewayNative with the same package ID.
Required actual-process assertions and success flags separately require GatewayNative
and AuthorityNative. Retained product binary checks also join their three exact
source roles before comparing executable paths/digests. Existing service cases
and required tests/counts are not weakened.

Role policy and fixture are decomposed into harness/packages/roles.rs; pure tests
move from the old inline module to harness/packages/tests.rs with same selectors.
One added test exercises SAME Runtime ID across five test contexts: only exact
matching role succeeds, other context/kind/target/source/duplicate metadata target
and old generic target cannot be admitted. Original short-ID/membership negatives
remain. Expected selected identity suite is now map3+helper2+catalog1=6, NOT the
previous source-specific five-test result. No test or production metadata load
was executed for this source. Model12 old archive evidence remains separate.

runtime-runner-consumer-source.json pins all10 changed/new Rust files. Core
source_inputs.rs remains SHA72190755582633f5289f1d3af541970b96cbbfb513c1fbbfd28403ee6e130f3c.
Bare source rustfmt, whitespace and document checks only. Root must allocate
metadata/libtest census, new six pure identities, source-guard selectors,
runner all-target check/strict Clippy and scoped fmt before consumer qualification.
Native actual parents, privileged Docker and journal/provider/Ready gates stay
open; no historical receipts rewritten, no accounting relaxation or CI dispatch.

## Immutable64 bounded role validation

Root allocated exact64f74c7ac64a8f5380172504bf73a8d03b961afc archive, no overlays,
fresh target/native-runtime-role-v0, jobs1/incremental0/locked/offline, ORIGINAL
390s work/450s total. Actual tool22048 terminal0 at98.29s, all processes terminal
and own marker removed within original horizon; target returned cache only.
Fmt FIRST0; three inventories EXACT3/2/1 before any selected tests; actual role
suite6 passed, zero failed/ignored/measured, filtered58/59/60 respectively.
Then standalone runner all-target check0/strict Clippy0. No source fix/retry.

Archive/source/tool/config3109 files and selected registry5594 files plus nine
environment keys were byte-identical before/after. Public
native-runtime-role-validation-summary.json pins51 raw attachments, archive SHA
c9fce9b071a5553f40db0bd7a39ae52c90fba76586ba28bbe0079953b86f54ab and exact
runner test executable1bca78b6d56f58c1 hash from all three Cargo test stderr logs.
The same-package sibling/wrong-kind/src-path/duplicate-role negatives executed
as pure fixture tests. Production Packages::load and actual repository/source
association remain NOT_RUN; native parents/privileged OS venues, whole Runtime
qualification, real journal/ACK/root/Ready proof are not established. Earlier
five-ID and twelve-model source-specific receipts remain separate history.
