---
id: plan:root/20261108030000
title: Concrete private CLI native runner consumer migration candidates
status: draft
created: 2026-11-08T03:00:00Z
updated: 2026-11-08T03:00:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/cli-installation, sot:root/structural-development, plan:root/20261107230000]
---

# Exact input and delivered candidate

Root requested actual Rust candidate changes in OWN evidence only. Read-only input:
.spec.local/root/evidence/20261108054000-core-consolidation/tools-integration-5f1010d4/
crates/apps/humanymous-cli/src/qualification/native_runner/. No live or Root-private
candidate source was edited. Nine revised Rust files, final-path unified diff and
exact input/candidate SHA256 plus test map are attached to evidence:root/20261106223747
under cli-consumer-candidate/. No manifest/lock/toolchain/dependency edit, compiler,
formatter, native process, account or service action. Source-only, NOT_COMPILED.

Files: evidence.rs; evidence/source_inputs.rs; harness.rs; harness/packages.rs;
harness/packages/{roles.rs,tests.rs}; harness/{build_observation.rs,helpers.rs};
service/catalog.rs. native-cli-consumer.patch uses final CLI paths and exact input
blob preimages; Root must compare inputs before applying, not overwrite newer work.
source-and-test-map.json preserves all61 original source-case IDs/hashes/selectors
and exact new native_runner:: selectors from the CLI map. All18 #[test] function
names present in the nine changed files remain identical. Other43 source cases
are unchanged by this candidate. No new/removed test function or selector alias.
This is static mapping, not actual compiler --list or execution.

# Source closure and one real lock scope

evidence.rs replaces old crates/cli and standalone runner scans with complete
crates/apps/humanymous-cli, crates/sdk and Platform package trees. Regular files
of ALL extensions are hashed, preserving included JSON/assets beyond Rust source;
target/.git/.spec.local are skipped and links/nonregular entries refuse. Workspace
mode still enumerates actual Cargo workspace members; no extra old runner scan.
No generic CARGO_MANIFEST_DIR root discovery or unrelated directory fallback.
Current working directory must satisfy the explicit required paths.

Both inventory and supplied-snapshot verification require11 explicit inputs:
the three existing Core manifest/compiled-JSON inputs; root Cargo.toml/Cargo.lock/
rust-toolchain.toml; Runtime/Platform/CLI manifests; runner NEW thin root and actual
private implementation mod.rs. The three existing source-input tests now exercise
all11 omissions/mutations and derive fixture count from the input set. Historical
snapshot bytes are not rewritten or accepted as newgraph source qualification.

New source schema humanymous.native-runner-source/2 identifies workspace-root-only
lock scope and exact runner composition/implementation/feature recipe as DATA,
with authority_proven=false. Build input/observation schemas advance to /2, root
lock occurs once, CLI manifest/thin-root/implementation hashes are separately
required. Missing/null old runner lock never becomes freshness. Public projection
has no runner_lock_sha256 field. input_key_complete/cache_reuse_proven remain false;
all dependencies/compiler/SDK OS closure is NOT claimed. New scope labels say
runtime-cli-native-process-component; historical receipts remain unchanged.

# Exact artifact and role joins

harness artifact parsing retains executable hash, opaque package ID, target and
kind, adds actual Cargo target src_path, profile.test and compiler features. Reject
missing role observation fields and conflicting observations for one executable.
No filename/package-substring match. Packages::load still requires same-source
metadata, exact workspace membership/local package version/manifest hashes and
source before/after verification. Production loader execution remains NOT_RUN.

Role matching now also requires exact compiler source path and profile.test,
actual feature array, and applicable required-features in SAME metadata target.
CLI path is crates/apps/humanymous-cli. Service test roles become
cli-service-process -> src/qualification/tests/cli_service_process.rs and
cli-runtime-service-process -> src/qualification/tests/cli_runtime_service_process.rs.
Service feature/selector/case ID/failure/ignored accounting remain unchanged.
Runtime/Platform target/kind/source roles stay exact; shared CLI ID cannot merge them.

Normal RunnerBinary and RunnerHarness share CLI bin target/name/thin-root and
qualification-native-runner eligibility but require profile.test=false versus
true. Production CLI/library, Lab or other bin cannot match either exact tuple.
The three existing package tests now use complete fixture entries and extend
same-package negative checks: wrong source/profile/package/feature, Lab feature,
metadata source swap, wrong executable digest and unit-harness-as-actor refusal.
One synthetic file verifies compiler row/digest joins; it is NEVER executed and
cannot prove native actor identity. Helper and service catalog existing tests
use the same complete fixture entries; no helper-ignore/count relaxations.

Optional source.runner_artifact is an external compiler-ledger OBSERVATION:
if present, exact normal runner role/path/digest is checked; absent data is never
filled from current_exe. This optional data is not authenticated launcher origin,
does not grant actor dispatch and changes no actor behavior. A forged JSON row
cannot create authority through this evidence-only join. Source carries explicit
authority_proven=false. NativeProof/ServiceHost/doctor/install trust stay untouched.

# Deliberate integration gates outside these three contexts

Existing harness preparation commands remain source-bound native-service features
and broad --bins/--all-targets. Do NOT add qualification-native-runner blindly:
building its normal bin internally could rebuild the currently executing runner
and hit Windows image locks, and broad target selection can alter denominators.
Root must freeze the final exact graph/target recipe separately. Candidate does
not exclude unrelated cases or credit runner61 as executed; source-map61 remains
a required semantic inventory reconciliation after approved compilation.

Actor self-role is NOT qualified by this candidate. Root/Delivery's external
launcher must retain actual normal runner compiler ledger plus source/feature/
artifact association, then separately accepted actor-path wiring validates it
before dispatch. current_exe stays a locator, never authority. No actor.rs/SCM/
unsafe allowance is changed. Fail closed on missing required actor association;
the optional observation is not a substitute for that owning admission boundary.

Controlled watchdog prefix still requires separate owned update to
cli_watchdog_boundary- with its existing exact selector/hex/.exe shape. It lies
outside these requested contexts and is not silently edited. Historical old
binary names/receipts and other-context negative fixture labels remain literal.

# Exact validation handoff

NOT_RUN: fmt/check/Clippy/compiler list/all61/native loader/actor/hosted ServiceHost.
Root may first apply to a fresh isolated matching input, formatter FIRST, locked
scoped check/strict Clippy, semantic --list against61 retained rows, then exact
existing tests below under native_runner:: prefix:

- evidence::source_inputs::tests::nested_manifest_and_each_compiled_input_mutation_changes_inventory
- evidence::source_inputs::tests::each_missing_required_input_refuses_inventory
- evidence::source_inputs::tests::each_omitted_required_snapshot_entry_refuses_verification
- harness::packages::tests::short_id_requires_exact_member_name_manifest_and_target_join
- harness::packages::tests::metadata_nonmember_duplicate_registry_and_missing_version_cannot_assign_role
- harness::packages::tests::shared_runtime_id_requires_exact_context_target_kind_and_source
- harness::helpers::tests::exact_helper_owners_do_not_include_product_host_pilots
- harness::helpers::tests::ignored_privileged_parent_is_required_and_never_a_verified_helper
- harness::build_observation::tests::projection_excludes_private_paths_flags_and_untrusted_labels
- service::catalog::tests::required_cases_match_exact_owned_package_target_and_test_kind

These10 selected existing functions cover meaningful source/role/schema changes;
the JSON map lists all18 unchanged functions in changed files and all61 original
IDs. Actual production Packages::load, normal runner source association, required
ignored parents and OS/process pilots remain separate actual gates. D2-D6 Native
observer observations/decisions remain separate; no proof minting or product pass.
