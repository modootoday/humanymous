# Actual CEF caller correction against Root's frozen assembly

This correction supersedes **only the caller candidates** in d340d7a3 and
34f0af03. Their original attachments/private trees remain immutable. The corrected
Browser model/native composition candidate remains a separate Root adoption.

Root found an actual implementation defect before adopting any caller: path
separator differences bypassed the intended main/provisioning transformations.
The earlier patches were applicable, but their source still built the same merged
package twice without role features or immediate artifact preservation. Source
snapshot entries also retained retired manifests. The prior integration prose
therefore overstated those caller changes. Applicability alone did not establish
their behavior. This record preserves that failure and supplies new actual Rust.

## Exact adoption boundary

`root-cef-callers-correction.json` binds seven **current Root isolated** raw
preimages and their complete candidate postimages, not the old standalone tool
preimages. `root-cef-callers-corrected.patch` addresses
`crates/apps/humanymous-cli/src/qualification/cef_pilot/` directly. Complete source
files and raw preimages are preserved in
`.spec.local/root/evidence/20261108131000-browser-ui-cef-caller-correction/`.
The frozen Root tree is recorded in the JSON; it was read only. All seven source
hashes still matched that tree when the patch was generated. `git apply --check`
passed against those exact preserved preimages. No patch was applied.

Root may adopt these seven postimages only after checking that exact preimage
mapping. Do not reapply the superseded standalone caller patches. Root's entry
visibility, existing module nesting and `cef_pilot` composition remain intact.
No live source, common manifest, lockfile, workflow or contract replica was edited.

## Implemented candidate behavior

The driver creates a fresh `native-role-artifacts` directory. A sequential loop
first builds `humanymous-browser --lib --no-default-features --features
cef-bootstrap`, records its own actual command/log receipt, and immediately
preserves `humanymous_browser.dll` as `humanymous_cef_bootstrap.dll`. It then builds
`cef-desktop` with the same explicit library/default-feature controls and preserves
that DLL separately. An error in either build or preservation exits before the
next step. An existing staging directory or staged filename is refused.

`copy_new_verified` rejects non-regular/linked sources, opens the destination with
`create_new`, copies/syncs it, and compares before/after source hashes, destination
hash and copied byte count. The driver stores actual staged hashes/sizes for each
role. These files stay outside Cargo's overwritten library output. A fresh runtime
bundle copies the two preserved role files, pinned SDK runtime resources and
upstream bootstrap/manifest. Fresh positive/cancelled case bundles reuse the same
SDK resource-copy implementation. Final packaged role hashes must equal their
individual preservation receipts before the runner can report success. This is
source implementation, not evidence that a DLL was built or copied in this turn.

All six protocol/process Cargo test invocations now select the merged package,
explicit role plus `native-pilot-tests`, `--no-default-features` and `--lib`.
Bootstrap/desktop exact selectors and test names match the private native module
identities. Existing real process cases, rejection counts, deadlines, cleanup,
standard-hosted Windows x64 refusal and unqualified GUI/Ready flags remain.

The source snapshot now names the merged Browser/CLI/Runtime/Core/Platform
manifests and source roots, root Cargo/lock/toolchain, Browser producer contract,
and packaged CLI qualification data. It contains no retired Browser package or
standalone CEF tool manifest/lock/source entries. Literal snapshot paths were
checked to exist in Root's assembly. This is a declared-source snapshot, not a
complete external SDK/dependency closure or transfer of Root's earlier source seal.

## Receipt schema and contract custody

The private report declares `humanymous.cef-native-pilot/2`. Safe diagnostics now
declare `humanymous.cef-safe-diagnostics/2`, expose separately sanitized
`native_builds.cef-bootstrap` and `.cef-desktop`, and allow both role build/staging
failure stages. Missing build receipts stay null. Staged hashes/sizes are
independently projected through a fixed role allowlist. The old single
`native_build` projection is removed; no aggregate success is inferred. Root or
Delivery must explicitly migrate any external version-1 consumer and reject an
unexpected schema/missing role receipt. No external consumer qualification is
claimed by changing this producer.

The compiled contract still comes from CLI's existing package-local
`qualification/browser/cef/pilot-contract.json`. Its include path is unchanged;
there is no out-of-package producer include. Neither the contract replica nor its
immutable provenance was refreshed. Root must commit/review the new producer and
update the replica/provenance through its accepted equality process. Until then,
the new selectors deliberately refuse the old contract instead of silently
accepting stale test identities. The receipt records the observed replica and
provenance hashes without relabeling their historical producer.

## Verification and remaining gates

The actual function bodies were inspected after editing. Bounded source checks
verify explicit role features, sequential immediate preservation, six role test
feature selections, seven no-default flags, removal of retired snapshot roots,
create-new/hash checks, packaged hash joins, schema separation and preserved CLI
contract include/entry visibility. These checks and patch applicability passed;
they do not establish Rust syntax/type correctness or execution behavior.

Three meaningful Rust regressions were authored: preserve the first role while
the Cargo output changes and refuse reuse; preserve a desktop failure independently
of bootstrap success while sanitizing private fields; keep unexecuted role receipts
null and retain a staging failure. None ran. No formatter/compiler/Cargo metadata,
unit test, Clippy, SDK provisioning, process/browser pilot or release ran.

Root's independently measured pure-model tests and feature graphs do not qualify
these new callers, native role binaries, the refreshed contract or a combined
receipt. Both SDK role builds and actual hosted pilots remain required. Real CEF
keyboard/IME/candidate/AX/DPI/sandbox/UI/performance and distribution gates remain
open, as do separate Core/Gate shipping-role contracts. Rust-only and mandatory
sandbox constraints are unchanged.
