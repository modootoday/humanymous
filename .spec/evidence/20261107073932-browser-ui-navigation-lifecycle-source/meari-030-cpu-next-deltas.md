# CPU candidate checkpoint and exact next integration deltas

Root owns the separate `meari-030-cpu-review-only` assembly. Its native-ui Cargo
check completed with the 31.90s successful footer; Root reports exit zero. Strict
Clippy completed exit zero with a 3.07s footer, and default library tests completed
32 passed, zero failed/ignored/filtered. No native-ui document tests or CEF role
binaries were executed by these commands. These are actual Root results, not the
864 upstream test report and not a run performed by this Browser delegate.

The attached JSON binds the exact logs/checkpoint/tree/source-map bytes. The
1,802-entry before/after maps match with zero difference records. Their timed
checkpoint covers strict Clippy/default tests. The initial check log predates that
pair; retain its observed successful footer and Root-reported exit separately,
without claiming the later snapshots bracketed it. No source-closure claim covers
SDK/cache/bundle/process state or a future merged source epoch.

The Windows normal/build native-ui tree selects seven Meari 0.3.0 family packages
at exact revision `ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4`, and has zero selected
wgpu, GPU Vello or non-CPU anyrender_vello rows. This is a selected graph, not a
hardware or window observation. The lock-resolution log records seven family
replacements and removal of windows-sys 0.59.0; final tuple/checksum review remains
Root's gate. Primary is not upgraded. CPU/window defaults, CEF pins and sandbox
requirements remain unchanged.

## Current layout and source identity

Use the accepted eight-family paths below. Root's intermediate workspace still
has 19 members; this list does not claim all eight-family migration is complete.

| Owner/context | Exact paths and next delta |
|---|---|
| Root graph and Browser | `Cargo.toml`, `Cargo.lock`, `crates/apps/humanymous-browser/Cargo.toml`: review/adopt the three direct Meari revision changes together and intentional seven-package lock replacements, then establish a new source/metadata/lock epoch. Keep normal SDK/default and bootstrap isolated from UI dependencies. |
| Browser native implementation | `crates/apps/humanymous-browser/src/native/bootstrap/mod.rs`, `src/native/desktop/mod.rs`, `src/native/desktop/host.rs`, `src/native/shared/scheme.rs`: no CPU renderer API rewrite is indicated by the successful compile. Preserve exports, same shared scheme source, borrowed sandbox pointer and Meari checked ownership. Actual CEF compilation and execution remain pending. |
| CLI CEF composition | `crates/apps/humanymous-cli/src/bin/cef_pilot/main.rs` and `src/qualification/cef_pilot/{mod.rs,provisioning.rs,diagnostics.rs,desktop.rs}`: preserve Root's adopted artifact-ledger join and exact actual output-path copy. Rejoin final package ID/manifest/source path/features/metadata and command ledger after any pin adoption; do not reuse an old join seal. |
| Runtime/platform dependencies | `crates/sdk/humanymous-runtime/` and `crates/integrations/humanymous-platform/`: snapshot their final source together with CLI/Browser/Core. This CPU upgrade grants no runtime feature, nativeproof, Ready, authority or policy change. |

## Contract and versioned consumer delta

The current producer is
`crates/apps/humanymous-browser/native/desktop/pilot-contract.json`. Its bootstrap
test is module-prefixed `bootstrap::tests::...` and desktop protocol selector is
`desktop::pilot_assertions::tests::`. The package-local CLI replica at
`crates/apps/humanymous-cli/qualification/browser/cef/pilot-contract.json` still
has the old unprefixed identities. Hashes and literal values are attached.

1. Root must first commit/review the current producer bytes, then copy that exact
   committed Git blob into the reviewed CLI replica. Preserve protocol counts,
   existing payload schema versions and negative cases. Changing module selectors
   does not automatically justify inventing new navigation/summary wire versions.
2. Update `qualification/data-provenance.json` through the accepted explicit
   refresh process: exact new producer path, source commit, Git blob, SHA256 and
   bytes for L07. Retain the former producer receipt in history; seven unrelated
   replicas keep their immutable identities. Do not change their source commits
   merely to make a common comparison pass.
3. The concrete blocker is
   `crates/apps/humanymous-cli/src/qualification/delivery/qualification_data.rs`:
   `INPUTS` still names the retired Browser source path and `records()` requires
   every replica to equal the single historical `COMMIT`. Replace that common pin
   with an explicitly reviewed per-replica source/commit allowlist. Check the new
   L07 against its committed producer while leaving the other seven pinned.
   Never trust arbitrary producer commits supplied by the provenance file itself.
4. Update the module's meaningful refusal/equality/packaged-data tests for mixed
   reviewed producer epochs, duplicate/foreign inputs, wrong bytes and stale L07.
   Keep eight required replicas and fail-closed validation; do not optionalize L07.
5. CLI emits private `humanymous.cef-native-pilot/2` and safe
   `humanymous.cef-safe-diagnostics/2`. Versioned consumers must require both role
   command/artifact receipts, preserve missing/failed values and accept no old
   single `native_build` aggregate. The bounded current-code search found the
   versioned producer/tests; it does not prove every external artifact consumer
   has migrated. Delivery/Root must finish that inventory before promotion.

## Workflow, report custody and exact command family

Delivery owns `.github/workflows/cef-native-component.yml`; the live workflow
still triggers old Browser/tool paths and calls the standalone manifest. Its
reviewed successor must trigger the merged Browser path, CLI CEF source/bin,
reviewed CLI contract data, root Cargo/lock/toolchain and the workflow itself.

The current merged target is package `humanymous-cli`, bin `humanymous-cef-pilot`,
feature `qualification-cef-pilot`. Replace the standalone test/lint/run dispatch
with that exact package/bin/feature under `--locked --no-default-features`;
preserve the standard hosted Windows job and all protocol/rejection/positive/
cancelled cases. No command is executed by this delta list.

`cef_pilot::emit` retains a private full `report.json` and emits its safe projection
to stdout. The live workflow uploads `report.json` when its artifact toggle is
enabled. The new private `cargo` join now includes actual paths and ledger identity:
Root/Delivery must explicitly qualify report custody and use the safe schema-2
projection for any ordinary shareable artifact. If a file artifact is required,
add an actual safe-projection file writer and point the reviewed upload at that
file. Do not rename a private file into a public receipt or publish the raw ledger.
Private diagnostics and their retention/access contract remain separate.

## Registry, SBOM and notices coordination

Root keeps Git source identity for this candidate. Registry 0.3.0 observations
remain a separate source view: do not substitute crates.io checksums for Git
packages or assert archive/tag equality without checking it. Delivery should
produce three explicit inventories tied to the adopted lock/source epoch:

- Full lock/dependency inventory, including optional and target-specific entries.
- Selected Windows CPU normal/build role inventory and exact feature graph.
- Actual distributed CEF/Meari/native bundle files, notices and upstream resources.

Update Meari-family version/revision provenance and the patched meari-shell
baseline/maintained-source notice. Retain the appropriate MIT OR Apache-2.0 license
materials and CEF/Chromium notices separately; our package LICENSE is not proof of
all dependency notices. Review windows-sys 0.59.0 removal against each inventory
before removing its distribution attribution. A bounded search of current root
NOTICE/THIRD_PARTY_LICENSES and Browser NOTICE found no literal Meari pin entry;
therefore regeneration/coverage review is required rather than claiming that one
version-string edit completes notices. Keep GPU-only inventory out of the selected
CPU success denominator while recording any lock-wide optional packages honestly.

## Ordered qualification still required

After the reviewed producer/replica/consumer/workflow/notice changes and final
epoch join, Root allocates role tests/strict checks, exact locked SDK role builds
and PE export/import inspection. The artifact ledger must select and immediately
preserve each actual role output before the other role build; caches/stale or
unjoined outputs stay refused under the current candidate contract.

Then execute the real sandbox-required CEF bootstrap rejection, positive/cancelled
process pilots, navigation/tabs and full CPU shell: dev/release resize behavior,
crowded tabs, SVG/font/cursor visuals, borderless gutters, max/restore/minimize,
parent/child bounds and DPI, keyboard/focus, Korean composition and measured Hanja
candidate placement/selection, AX actions and descendant shutdown. Full sandbox
policy observations retain unknown/unsupported cases. Actual display latency is
not inferred from present-return or CPU compile success. GPU remains a separate
unqualified opt-in candidate and no scoring or external-navigation policy changes.

This document is a source/contract delta assignment to the named owners. The
Browser delegate ran no compiler, test, SDK, PE, window/process/browser pilot or
deployment, and changed no shared implementation, manifest, lock or Root assembly.
