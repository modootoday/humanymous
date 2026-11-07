---
id: sot:root/native-embedding
title: Opt-in native SDK and DLL embedding
status: active
created: 2026-11-07T04:10:00Z
updated: 2026-11-07T04:35:44Z
verify: manual
rel:
  governs: [crates/native-cli/**]
  depends_on: [sot:root/rust-migration, sot:root/chromium-lab-browser, sot:root/browser-ui]
---

# Opt-in native SDK and DLL embedding

## Rule

Version 0.3.0. User-approved extension of the laboratory browser's observation
model into developer-integrated native desktop applications. This is an accepted
design, not implemented SDK support or evidence of automation blocking. Rust-only,
detection freeze, accessibility and mandatory sandbox boundaries remain binding.

SDK-01: The app developer explicitly initializes a Rust SDK or loads a Windows
cdylib through a versioned C ABI. Arbitrary process injection, global keyboard
hooks, DLL search-path modification, privilege escalation and third-party probing
are outside scope. Register only host-owned windows and approved capabilities.

SDK-02: Planned foundation crates are humanymous-native-core (facts/capabilities/lifecycle),
humanymous-native-windows (owned-window OS adapter), humanymous-native-ffi (thin
versioned ABI) and humanymous-native-plugin (host registration/callback adapter).
Reuse existing scoring, audit and cryptography; do not duplicate policies or
engines. Actual crate creation/workspace registration requires its implementation
slice and checks. Naming, public rustdoc, module/file limits, reviewed unsafe and
testing follow sot:root/rust-migration and AGENTS.md.

SDK-03: Non-owning lifecycle is Uninitialized -> Initialized -> Running -> Stopping
-> Closed or CleanupIncomplete. Use generation-scoped opaque handles; define UI
thread, worker, reentrancy and callback ownership. Repeated init, stale handles,
shutdown during callback and deadline expiry return explicit errors. Stop revokes
authority before cleanup; cleanup completion is separately observed. Never exit
the host process or silently retry an uncertain side effect. No work in DllMain
beyond minimal loader-safe setup; start IO/workers only after explicit init.

SDK-04: ABI negotiates version, struct size and capabilities. Use fixed-width
integers, explicit lengths and bounded UTF-8 buffers, explicit allocation/free
pairs and errors with documented data lifetime. No Rust String/Vec/trait-object/
enum layout crosses the ABI. Catch panics at each exported boundary; prohibit
unwinding across foreign calls. Unload is forbidden while SDK workers or callbacks
remain live. A Rust-authored consumer must test the actual dynamic ABI.

SDK-05: First release is opt-in observation only. Reuse research provenance and
visibility planes; distinguish requested/delivered/observed/unknown. A DLL cannot
certify human identity or universal Computer Use/CDP detection. Enforcement needs
a separately canonical, explicitly authorized policy at the host's sensitive
operation boundary. Required evidence missing means that protected operation is
fail-closed, not whole-app termination or blanket input suppression. SDK addition
does not authorize scoring weights, thresholds, predicates or verdict changes.

SDK-06: AT/UIA presence, injected-input markers, input speed and motor richness
cannot individually gate Pass/CHALLENGE. Qualification includes legitimate AT,
remote desktop, physical input and authorized test automation controls. Preserve
the T4 ceiling and uncertainty; compromised in-process hosts remain a trust limit.

SDK-07: Loading an SDK is no sandbox attestation. Declare OS/architecture/runtime/
privilege capabilities and host/child trust boundaries. CEF adapters retain the
existing mandatory sandbox and unknown-deny requirements; do not reuse the browser
bootstrap's borrowed sandbox pointer as generic proof. The current experimental
RunBrowserDesktop owns CEF initialization/shutdown and calls process::exit on
failure: it MUST NOT be renamed/re-exported as an embeddable SDK entrypoint.

SDK-08: Default retained/exported data excludes raw keys, AX/window text,
screenshots, clipboard, credentials and personal process inventories. Observe
minimal app-owned metadata only. Sensitive capture/export requires per-run purpose,
scope, expiry, revocation and destination. Bound queues, loss counters, CPU/memory
budgets and shutdown time; expose unknown/lost data without inventing clean results.
Erasure/kill-switch/permanent-ban dual-control rules remain binding.

## Why

Blue/red review identifies host-process exit and in-process tampering as P0;
SDK-01/03/05/07 separate embedding from process-owning experiments. SRE requires
bounded stop/unload and callback lifetimes in03/04/08. Privacy requires minimal
app-owned capture in01/08. Evaluator requires actual DLL consumer and AT controls,
not promotion of browser fixture evidence, in04/05/06. These findings were checked
before SDK implementation; all SDK runtime and enforcement coverage is not_run.

## Examples

A host registers its own window, starts metadata observation, requests a bounded
snapshot and stops before unloading. No protected action is automatically blocked.
A stale handle returns an error; a missing required proof leaves an explicitly
configured protected action unavailable. Neither case terminates the host.

## Verification

Follow plan:root/20261107041001. ABI fuzz/negative inputs, panic containment,
concurrency/reentrancy, stop/unload, legitimate AT and input cohorts, sandbox,
signing/distribution, privacy/loss/performance and cross-OS remain unverified.
Current browser IME/native-child evidence does not satisfy these SDK gates.


## Framework adapters and create tooling

SDK-09: Separate the framework-independent foundation from opt-in thin adapters.
Framework adapter names below are planned packages, not support promises. The
native CLI is separately implemented only to the bounded manifest scope below.
Framework adapters reuse SDK-01..08 and never fork scoring, audit, crypto or an
engine. Host language does not change the observation/enforcement distinction.

| Planned integration | Boundary | Initial status |
|---|---|---|
| humanymous-native-electron | Rust Node-API addon loaded by trusted Electron main process | planned/not_run |
| humanymous-native-tauri | Rust backend plugin and explicit scoped capabilities | candidate/not_run |
| humanymous-native-winit | Rust owned-window/event-loop adapter, reusable by Meari consumers | candidate/not_run |
| .NET/WPF/WinUI, Qt, Flutter consumers | Versioned native-ffi C ABI, host-owned binding/lifecycle glue | candidate/not_run |
| humanymous-native-cli (bin humanymous-native) | Rust integration planning/scaffolding CLI | first implementation slice below; adapters not_run |

SDK-10: Electron loads a separately versioned Rust-authored .node addon through
Node-API; the generic C ABI DLL is not itself require-able as a Node addon. The
trusted main process owns initialization, registered windows and shutdown. Never
load SDK native code into sandboxed renderers or demand unsandboxed preload.
Require sandbox=true, contextIsolation=true, nodeIntegration=false and deny
security-weakening switches. Do not treat these settings as a measured sandbox
policy attestation. Any renderer API is optional and uses bounded typed messages,
validated actual webContents/frame/origin/session and current window generation.
Revoke renderer authority on navigation, reload, crash or destruction; never trust
a renderer-supplied window ID or claim of being the main process. No raw IPC, HWND/PID,
filesystem, command, library-loading, admin or SDK kill-switch authority crosses
the renderer boundary. Unknown sender or revoked generation is denied.

SDK-11: Consume the host's existing engine; do not initialize a second CEF or
Chromium inside Electron/Tauri. Electron public host callbacks, app-owned OS facts,
CEF internals and page-visible evidence are different capabilities/provenance.
A Node-API addon or a native window handle does not grant Blink/V8-internal
observations, prove actual input delivery or establish website-visible detection.
Declare unavailable hooks explicitly. CDP/debugger attachment must be a separately
admitted observer with contamination recorded, never silently enabled at startup.
Tauri/WebView, Qt/Flutter and plain winit consumers do not inherit CEF qualification.

SDK-12: Node-API stability is not an all-Electron-version compatibility guarantee.
Publish support only for measured tuples: SDK/API/Node-API, exact framework and
embedded runtime versions, OS/architecture/toolchain, privilege/sandbox policy,
dev versus packaged release and artifact hash/signature. Test actual .node loading,
export negotiation, owned-window registration, event-loop/nonblocking callbacks,
Node environment teardown/reload, app quit/window recreate, stale generations,
packaging/native-library resolution (including ASAR unpack when applicable) and
dependency compatibility. Validate native handle width, owning PID/thread and
lifetime; missing platform handles mean unavailable capability. Do not replace
the Electron event loop or start another winit loop. Node teardown must cancel
thread-safe-function callbacks/workers and separately report cleanup. Do not claim
that catch_unwind contains arbitrary native memory faults. Main-process compromise
and malicious in-process code remain trust limits. Unload needs cleanup receipts.

SDK-13: The humanymous-native init command accepts an explicit framework and target
and defaults to a read-only dry-run plan. Outputs are versioned integration/
capability manifest, bounded configuration, Rust scaffolding and human-readable
host-side requirements. Do not emit JS/TS/C++/C#/Dart application code, embedded
scripts, shell installers or foreign-language templates. Required consumer glue
belongs to the external host project and is declared, not called Rust merely
because generated by a Rust program. No zero-glue integration or working Electron
consumer is claimed without a conforming tested host integration. An unqualified
framework/tuple may produce a plan marked unsupported/not_run, never a ready badge.
No dependency pin is invented before implementation research and tests.

SDK-14: Scaffolding never silently modifies existing projects, runs package-manager
lifecycle hooks, downloads/executes binaries, enables telemetry, imports profiles,
changes security settings or attaches to processes. An explicit write operation must
show exact bounded destinations and changes, stay inside the explicitly selected
workspace, reject traversal/reparse escapes and refuse overwrites. Creation must
fail atomically without leaving a partial ready-looking integration. Deterministic
outputs include schema/tool/dependency versions, capability defaults and rollback
instructions. Initial observe-only/default-deny policy remains. An artifact being
created or loaded does not imply signal availability, sandbox or enforcement.

## Framework critique reflected

P0 renderer-to-native privilege bridge -> SDK-10. P0 Electron host calling the
process-owning CEF pilot ->07/10/11. P1 stable ABI mistaken for universal support
->12. P1 create generator laundering authored foreign scripts as Rust ->13/14.
P1 destructive project mutation/install hooks ->14. Blue/SRE lifecycle and privacy
checks ->10/12/14; evaluator hook/provenance and support-matrix boundaries ->09/11/12.
All framework runtime tests are not_run; bounded native CLI component execution
is recorded below and does not establish SDK/framework compatibility. Current
browser native window evidence is not promoted to framework compatibility evidence.

## Primary references for framework design

Checked2026-11-07 KST; these are upstream API facts, not executed compatibility tests.

- Electron native modules: https://www.electronjs.org/docs/latest/tutorial/using-native-node-modules
- Electron sandbox/process and preload boundary: https://www.electronjs.org/docs/latest/tutorial/sandbox
- Electron IPC/security constraints: https://www.electronjs.org/docs/latest/tutorial/security
- Node-API ABI stability limits: https://nodejs.org/api/n-api.html
- Tauri Rust plugin/capability lifecycle: https://v2.tauri.app/develop/plugins/
- Flutter foreign-function boundary (candidate only): https://docs.flutter.dev/interop/bind-native-code


## Convention-aligned native CLI implementation slice

SDK-15: Package name is humanymous-native-cli, executable humanymous-native,
initial command init. No create-prefixed alias or second implementation. Future
commands doctor/verify are reserved design only, not accepted silently. Source
lives in crates/native-cli with workspace conventions and forbid(unsafe_code).

SDK-16: The first independently useful tool emits one deterministic versioned
JSON integration manifest. It does not need the still-unimplemented SDK crates:
framework/target/runtime selection, generator version, planned adapter, requested
observe-only/sandbox-required policy and unavailable capability/qualification
states are declarative. init defaults to stdout dry-run; --dry-run and --write
conflict. Require --framework; refuse duplicate/unknown options and unknown
framework/target names. Bound argument sizes; refuse option-looking tokens as
values so --dry-run cannot be swallowed as a version and bypass mode conflict. No personal data, timestamps,
credentials or automatic machine/process/profile inventory enters the manifest.
No Rust scaffold with nonexistent SDK imports or foreign-language glue is emitted.

SDK-17: --write requires an explicit existing absolute --workspace and publishes
one new direct-child JSON basename (default humanymous-native.json). Refuse path
separators, traversal, alternate streams, DOS device names, existing targets and
non-JSON output names. Initial write support is Windows local drive directories
only; other OSes retain dry-run and explicitly refuse write. Reject every reparse
ancestor and hold opened directory handles without delete-sharing through the
operation, so ancestors cannot be renamed/replaced during path-based writes.
Read-capable ancestor handles are required; access0 metadata handles did not
prevent rename in the actual test. Reject network/substituted drive aliases after
pinning, and deny write/delete sharing on the temporary file.
Create a fresh same-directory temporary file with create_new, write/sync complete
bounded bytes, publish using atomic hard_link (no replacement), then remove only
the owned temporary link. No fallback to overwrite/copy on unsupported filesystem.

SDK-18: Atomic publication means the destination is absent or a complete manifest;
it is not a power-loss/directory-durability guarantee. Preserve explicit cleanup
status after publication and identify recoverable temporary names on failure.
No output ever asserts adapter installation, engine/sandbox qualification or
active protection. OS/platform write support and SDK/framework support are
independent. Workspace ancestors and entries must be trusted/non-mutating during the operation,
especially the pathname-based temporary cleanup after closing its handle;
this tool is not a defense against a malicious same-identity writer replacing
entries or mutating file contents. It never grants authority to a preexisting manifest. Tests include
concurrent no-clobber publication, static junction refusal, failure cleanup and
byte-identical dry-run/write manifests. Full host integration remains not_run.

SDK-15..18 supersede the previous create-tool name and allow manifest planning
before the core/FFI gates. Actual SDK, framework adapters and multi-file Rust
scaffolding still require their own implementation slices; no readiness inference.


## Implemented CLI frontier and critique reflected

Evidence:root/20261107042614 establishes Windows x86_64 native-cli init only:
11 unit and6 actual executable tests, deterministic dry-run for6 framework plans,
explicit complete manifest publication, repeated-write byte preservation and mode
conflict refusal with zero output. Release build/fmt/Clippy pass. The manifest
continues to say not_implemented/qualified=false for every adapter. Root registered
the tool as workspace member20; no foundation SDK/adapter package was added.

P1 option tokens swallowed as runtime versions -> bounded value parser plus actual
CLI conflict test. P1 metadata-only ancestor handles allowed rename -> read-capable
handles plus real Windows rename denial test. P1 postpublication cleanup ambiguity
-> separate receipt and real delete-sharing failure test. P1 same-user pathname
cleanup races/crash durability -> explicit trusted non-mutating workspace premise
and durability non-coverage, not fictional handle-bound erasure. Root static path/
junction checks and concurrent no-clobber publication are tested. Network/SUBST
mapping rejection is not an actual mapped-drive experiment; non-Windows compile/
execution, crash/power-loss, full SDK/DLL/framework consumption remain unverified.
