---
id: plan:root/20261107230000
title: Native runner consumer contract for private CLI qualification context
status: draft
created: 2026-11-07T23:00:00Z
updated: 2026-11-07T23:00:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/cli-installation, sot:root/structural-development]
---

# Scope and pinned inputs

Spec-only preparation for Root's Tools13 consolidation. No source, manifest,
lock, dependency, path movement or compiler execution is authorized here.
Root owns the final graph and source allocation after freeze. Journal/provider,
installer genesis and original-clock/native ownership goals remain OPEN.

Root's tools-Cargo.proposal.toml at private evidence directory
20261108054000-core-consolidation has SHA256
fcceb05ac0cb0143fee493e66fbaebcef60e42c485c75292b0dee36b787eac18:
27 targets, 17 default-off features. CLI checkpoint33de76a3 composition map is
.spec/evidence/20261107200932-cli-installation-package-transition-handoff/bin-composition-identity-map.json,
SHA256 8f50156afe03a5306db9464eaa31561e68b9cd6eff268d42332ed2a070a72ecb.
Its legacy-bin-unit-map.json SHA256 is
157af43efe37eda8d355966411699c397518967cd2e7ac43a1afd954dc5ec8c0.
Evidence file .spec/evidence/20261106223747-cross-os-runner-research/cli-native-runner-contract-source.json projects exactly61 native
source-case rows from that map; source discovery is not executed inventory.

# Private implementation and lint boundary

Final package humanymous-cli lives at crates/apps/humanymous-cli. Runner target
humanymous-native-runner has new thin root src/bin/native_runner/main.rs,
required feature qualification-native-runner. Actual old main implementation is
src/qualification/native_runner/mod.rs, private root namespace native_runner.
Every old Rust implementation file stays in that private context with actual
module relationships. Rewrite crate::Options/evidence/harness/actor/fixture/
service and other internal anchors to crate::native_runner::<context>.
Preserve CLI argv, error text, exit behavior and flush ordering through one
parent-visible command entry. Options can remain private to its subtree.
No forwarding SDK/helper library, production lib export or other bin may expose
actor capability. Thin-root hash and implementation hashes are separate pins;
old main's hash cannot be assigned to the newly authored composition root.

Package unsafe_code=deny; runner composition uses deny. Exactly the existing
module-level allows in actor.rs and actor/scm.rs remain, without new scopes or
broader allows. Other safe composition roots retain forbid. Proposed Platform
native amendments are separate unapproved work, not permission for this move.

# Closed artifact roles

A shared opaque package ID never supplies target authority. Require same-source
locked metadata, exact workspace membership, local source, owning manifest hash,
exact target name, kind and src_path, with one unique match. Record actual
compiler-artifact features and profile.test as well as executable path/digest.
Metadata required-features is eligibility, not proof of compiled features.
Current artifact collection omits these fields and must be changed under a
separate source allocation. Reject absent/duplicate/foreign-source joins.

| Role | Target | Kind | Package-relative source | Required feature | profile.test |
|---|---|---|---|---|---|
| Runner executable | humanymous-native-runner | bin | src/bin/native_runner/main.rs | qualification-native-runner | false |
| Runner unit harness | humanymous-native-runner | bin | src/bin/native_runner/main.rs | qualification-native-runner | true |
| Production CLI | humanymous | bin | src/main.rs | production recipe | false |
| CLI service pilot | cli-service-process | test | src/qualification/tests/cli_service_process.rs | service-pilot | true |
| CLI runtime pilot | cli-runtime-service-process | test | src/qualification/tests/cli_runtime_service_process.rs | runtime-service-pilot | true |
| CLI watchdog | cli-watchdog-boundary | test | src/qualification/tests/cli_watchdog_boundary.rs | runtime-service-pilot + service-pilot | true |

All these roles use final CLI manifest path; production CLI/library and every
other qualification bin are excluded from runner self-role by the exact tuple.
Actor current_exe canonical path/hash must join the normal runner executable,
its source/feature recipe and actual artifact before privileged dispatch. A
filename, package ID, source observation or caller boolean is insufficient.
Production association interface and actual loader pilot remain NOT_RUN.

Retain Runtime manifest crates/sdk/humanymous-runtime/Cargo.toml and exact roles:
humanymous_runtime/lib/src/lib.rs; gateway-native-process/test/tests/gateway/native_process.rs;
authority-native-process/test/tests/authority/native_process.rs;
audit-durable-witness/test/tests/audit/durable_witness.rs;
audit-key-backups/test/tests/audit/key_backups.rs;
audit-key-custody/test/tests/audit/key_custody.rs;
humanymous-runtime/bin/src/gateway/main.rs;
humanymous-authority/bin/src/authority/main.rs.
Platform stays crates/integrations/humanymous-platform/Cargo.toml,
humanymous_platform/lib/src/lib.rs. Shared CLI migration cannot broaden either.
Runtime WAL helper remains audit::storage::wal::tests::process_lock_probe under
the Runtime library role; test selectors do not transfer role ownership.

# Selectors, helpers and command admission

All61 old runner source selectors gain exactly native_runner::, including
prebuilt::tests and harness::packages tests. Preserve each cfg, ignored and
side-effect classification. Windows-only cases are not portable execution
counts. Compile/list the final graph to establish inventory before executing;
source map61 does not claim every venue runs61. Historical receipts stay intact.

Controlled CLI watchdog prefix becomes cli_watchdog_boundary-, exact selector
final_delivery_watchdog_with_retained_unread_pipes. Keep strict16lowerhex suffix
and .exe shape, no old alias. Platform capture/barrier prefix remains
humanymous_platform-, testing::process selectors unchanged. Timeout leaf remains
parent_remains_running_until_original_outer_deadline. Rename service catalog
metadata targets to cli-service-process and cli-runtime-service-process; retain
existing selector leaves, required-case identity, failure and ignored accounting.

Proposed runner preparation command recipe: cargo build --locked -p
humanymous-cli --bin humanymous-native-runner --no-default-features --features
qualification-native-runner --message-format=json --timings. Inventory/unit
recipe: cargo test --locked -p humanymous-cli --bin humanymous-native-runner
--no-default-features --features qualification-native-runner --no-run
--message-format=json --timings, followed by that exact harness --list and
approved exact selectors. Release mode must be recorded explicitly. These are
specification text, not execution or a resource allocation.

Native product builds must retain exact Gateway/Authority/production CLI/service
fixture targets and required service features in explicit batches. Broad --bins
or --all-targets after feature unification can select unrelated Tools contexts:
Root/CLI/Delivery must freeze a complete exact target/feature ledger before
execution. Workspace qualification keeps all required graph cases and ignores;
no silent exclusions, filtered-case credit or relaxed helper counts. Existing
service-pilot/runtime-service-pilot feature requirements remain package-qualified.

# Source custody and observation schema

Replace standalone runner directory/manifest/lock discovery with final CLI
package and workspace root Cargo.toml/Cargo.lock custody. Snapshot complete CLI
implementation plus all thin roots and owning manifest; retain Runtime/Platform
and three mandatory Core inputs already required by source_inputs. Verify
before/after actual source hashes; source observations grant no native authority.

build_observation input_key currently reads runner_lock_sha256 from the old
standalone Cargo.lock. Root/Delivery must version the schema to state one actual
workspace lock scope, pin root lock once and bind CLI manifest, thin root, native
implementation and exact feature recipe separately. Never substitute a duplicate
root lock under the old name or let missing/null old lock qualify freshness.
Update stale scope labels only in new observations; old receipts remain source
bound and immutable. source-key-complete/cache-proof stay false until the actual
complete input contract is implemented and validated.

# Review and verification boundary

Self-critique: package-only matching is P1 privilege confusion; test-harness/bin
confusion is P1; omitted feature/source joins and stale lock are P1 evidence gaps;
broad target selection or dropping ignored parents is P1 qualification drift.
The closed tuples, explicit recipes and pending schema/production admission gate
above are required remediation, not claims that current source implements them.

Six Runtime-role fixture passes at64f74c7a establish only those fixture contracts.
They do not execute production Packages::load, the new CLI graph or native actor
admission. Model12 and selector1 prior receipts retain their original source and
venue. No new test/typecheck/Clippy/native/Trivy/product success is claimed.
After source allocation: formatter first; scoped check/Clippy; exact semantic
inventory against61 source rows; negative role joins for same-package other bin,
lib and profile.test mismatch; production same-source metadata loader pilot;
then separately authorized actual OS/process pilots. Root owns compiler leases
and whole-graph gates. This draft is ready for interface review, not implementation
or production/native qualification closure.
