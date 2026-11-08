# Browser three-to-one transformation candidate

State: prepared, unapplied and uncompiled. Root alone applies the final workspace,
path, lockfile and Tools integration. This candidate does not allocate execution.
The source-map JSON records 49 Browser source/data/document/manifest preimages,
their Git blobs, destination paths and exact candidate SHA256 values. Private
preimages and full candidate trees are retained under
`.spec.local/root/evidence/20261108121000-browser-ui-three-to-one/`.

## Application order and ownership

1. Compare every source-map preimage hash with the integration source. Refuse
   drift; do not apply a patch by fuzzy matching a newer owner implementation.
2. Root moves the 47 entries with non-null destinations to
   `crates/apps/humanymous-browser`. The two nested package manifests retire;
   they are not copied. Nothing is duplicated into a private helper package.
3. Apply `three-to-one-rust.patch` at repository root after those moves. Its
   paths address the relocated tree. `git apply --check` passed against the
   preserved, relocated preimages; no product path was changed.
4. Root replaces the three workspace members with the one new member and
   resolves the locked graph. Preserve workspace version/edition/toolchain and
   dependency provenance. No independent lockfile candidate is supplied.
5. Rebase `three-to-one-callers.patch` onto Root's accepted Tools composition.
   This patch deliberately addresses seven preserved original Rust caller paths,
   not guessed post-move CLI module paths. Its independent preimage applicability
   check passed. Root's newer Tools source wins on drift.
6. Delivery updates workflow path triggers and command dispatch to the accepted
   CLI layout; retain the Windows hosted job and every mandatory pilot. Root
   refreshes the explicitly reviewed CLI contract replica and its provenance to
   the newly committed producer bytes. Historical producer records and native
   audit fixtures remain immutable; do not rewrite their source paths as history.

## Exact composition and artifact contract

The library retains the existing public model modules at crate root, each under
`forbid(unsafe_code)`. The composition root uses `deny(unsafe_code)` so that the
existing narrowly audited native `allow` scopes remain possible. Native source
roots become private `bootstrap` and `desktop` modules. Desktop-native references
are module-qualified; former external model references become `crate::` model
references. Shared scheme registration stays one source file compiled separately
for each process role. Existing relative source and data includes are preserved.

The default feature set remains empty and SDK-independent. `native-ui` retains
the management-only Meari binary. `cef-bootstrap` enables only CEF sandbox and
libloading; it rejects `native-ui`. `cef-desktop` enables `native-ui`, CEF sandbox
and the existing Windows host dependencies. Both native roles reject non-Windows
targets. Both roles together are an intentional compile error, not an all-features
qualification target. `native-pilot-tests` alone grants no native role.

Cargo builds one library target (`rlib`, `cdylib`) in separate invocations. Build
`--lib --no-default-features --features cef-bootstrap`, immediately preserve
`humanymous_browser.dll` as `humanymous_cef_bootstrap.dll`, then build the desktop
role and preserve it as `humanymous_cef_desktop.dll`. The caller patch uses
create-new/verified-copy semantics, refuses unknown roles, symlinks and divergent
existing destinations, and retains separate command logs. Use a fresh owned target
directory; stale bundles are refusals, never silently overwritten. Parallel builds
to the same target or assembling both role artifacts from the final DLL is invalid.

The bootstrap export remains `RunConsoleMain`; desktop remains
`RunBrowserDesktop`. The upstream bootstrap executable and its module selection,
fixed sibling desktop basename, search flags and borrowed sandbox pointer are
unchanged. Bootstrap still calls `execute_process` before application IO/UI loads.
`no_sandbox=0`, forbidden switches, pointer refusal, private synthetic profiles,
cleanup behavior and all unsupported capabilities/runtime flags remain unchanged.
Test names gain their actual `bootstrap::`/`desktop::` module prefixes; contract
counts, required cases, deadlines and failure checks are retained.

## Dependency and distribution boundary

Keep cef-rs revision `224a785c7b41758b51821cbb7395ea57ba493a63`, sandbox feature,
CEF 156.0.15 / Chromium 156.0.8078.80 archive identity and existing provisioning
checks. Keep Meari revision `1482b746ebc0d867f25d17d01c48c12df888ca6c` until an
explicit reviewed release update. A pending upstream GPU release is not an
installed backend, project support claim or permission to float the pin.
CEF binaries/resources/upstream bootstrap remain provisioned distribution inputs,
not embedded SDK downloads or authored foreign bootstraps. Preserve notices,
licenses, archive/provenance checks and the existing distribution qualification
frontier. This app is not an embeddable native SDK or public native ABI.

## Required Root integration checks

- Inspect default/native-ui/bootstrap/desktop dependency graphs independently;
  bootstrap must have no Meari/window dependency activation. Verify pure modules
  cannot opt out of unsafe prohibition and native unsafe scopes have not widened.
- Run locked formatting, scoped tests and strict Clippy for each supported role;
  verify dual-role, bootstrap-plus-UI and non-Windows role refusal separately.
  Do not use workspace all-features as the role matrix.
- Verify PE exports and dependency imports on both preserved DLLs, plus exact
  hashes joining each role build receipt to the packaged file. Prove subprocesses
  load the minimal role without loading the desktop/Meari DLL.
- Refresh the CLI's reviewed contract replica after producer commit. Its old
  immutable provenance does not qualify module-prefixed names. Update Tools
  source snapshot roots, diagnostic fixture paths, selectors and stage consumers
  against final layout. The candidate splits native-build into two stage receipts;
  any consumer requiring the old single `native_build` field needs an explicit
  schema migration rather than inferred success or a fabricated aggregate.
- Run all existing synthetic protocol cases with exact names/counts and the real
  hosted rejection plus positive/cancelled process pilots. Preserve failed first
  attempts, ignored/filtered denominators, unsupported tuples and actual teardown.
- Execute the unfinished real CEF navigation, keyboard, measured Hanja, IME
  composition/candidate placement/selection, AX, DPI, borderless resize and full
  applicable sandbox-policy pilots. The same-URL callback attribution limitation,
  UI-38 unexecuted tests and reported Meari resize latency remain open.

No compiler, formatter, Clippy, unit/process/native/browser/container execution,
SDK provisioning, GPU change, publication or release occurred for this candidate.
The previous native pilots do not qualify these new composition roots. Core/Gate
demo/static/WASM contracts remain a separate unfinished deliverable; this app
consolidation neither supplies those shipping roles nor changes scoring/Ready.
