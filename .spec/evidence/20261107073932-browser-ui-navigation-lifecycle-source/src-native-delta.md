# Standard-context native source correction

This explicit delta corrects the implementation layout of the immutable
`d340d7a3176417feeabd06b69fee177b0506610a` candidate. It does not apply product
changes, allocate a compiler/native pilot, or change the accepted package count.

`src-native-source-map.json` replaces the proposed destination mapping, not the
original preimages. All 49 original Browser records remain: 47 destination files
and two retired nested manifests. Every destination candidate hash is recomputed.
Each record also retains its d340 destination/hash so Root can inspect the exact
delta. The seven caller candidates retain their original raw/Git/preimage identity
and record both candidate generations. Original d340 patches, full private trees,
source map, pins and documentation remain unchanged.

## Corrected layout and semantics

- Minimal role: `src/native/bootstrap/mod.rs`, `renderer_probe.rs` and
  `negative_pilot.rs`.
- Desktop role: `src/native/desktop/mod.rs` and the existing desktop context files.
- Shared scheme: one physical `src/native/shared/scheme.rs` source file.
- Non-code contract: `native/desktop/pilot-contract.json`, unchanged from d340.

The root library composes the private `bootstrap` and `desktop` modules using
their corrected in-src paths. Those module names deliberately remain the same:
exports, crate-root model APIs, visibility, module-qualified Rust test identities
and role-feature/unsafe boundaries are unchanged from d340. Each native role root
references `../shared/scheme.rs`. Both native test readers now include
`../../../native/desktop/pilot-contract.json` from their corrected context.
The seven caller candidates retain d340 behavior and change source snapshots and
diagnostic fixture paths to the corrected layout. External contract paths stay in
`native/desktop/`; they are not confused with the moved Rust implementation.

This is the same CEF/Meari implementation, sandbox requirement and upstream pins.
No SDK, build role, combined success receipt, unsupported-platform behavior,
distribution support, default feature or runtime flag is added or upgraded.

## Choose one application route

For a fresh Root assembly from original preimages:

1. Relocate the 47 source-map entries to their **corrected** destinations; omit
   both nested Cargo manifests. Do not use the superseded d340 destination map.
2. Apply `src-native-rust.patch` to that relocated tree.
3. Rebase `src-native-callers.patch` from the seven original caller preimages onto
   Root's final Tools composition. Preserve exact contract replica/provenance and
   workflow/source-snapshot requirements from `three-to-one-integration.md`.

For a Root assembly that already contains the exact d340 candidate:

1. Apply `src-native-from-d340.patch`. It contains the actual native source
   relocation, root composition and include correction relative to that candidate.
2. Apply `src-native-callers-from-d340.patch` to the exact original-path d340
   caller candidates, or rebase its equivalent into the final Tools context.

These routes are alternatives; do not apply a complete patch and its delta to the
same tree. Each of the four patches passed `git apply --check` against its
corresponding preserved private preimage tree. No patch was applied to product
paths or private verification trees. Source/data/compiler identity must still be
checked against Root's actual assembly before adoption; applicability is not Rust
compilation or semantic qualification.

## Static checks and open gates

`src-native-static-check.json` records 42 Rust files, zero implementation files
outside `src/`, one physical shared scheme, and ten existing literal module/data
include targets contained within the candidate package. All 47 d340 destination
files and seven d340 caller files still match their recorded hashes. The new
private tree is `.spec.local/root/evidence/20261108124000-browser-ui-src-native-delta/`;
original raw preimages remain in the separately preserved d340 private tree.

No Rust formatter/compiler, Cargo metadata/resolution, test, Clippy, process,
browser, SDK provisioning or release ran. All per-role graph/PE/packaging/hosted
pilot gates in `three-to-one-integration.md` remain required. Core/Gate web-role
contracts and unfinished real CEF/UI/keyboard/IME/AX/DPI/sandbox/distribution goals
remain active and unqualified. Root alone owns final workspace/lock/Tools merge.
