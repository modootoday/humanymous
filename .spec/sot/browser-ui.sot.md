---
id: sot:root/browser-ui
title: Native browser UI and CEF boundary
status: active
created: 2026-11-07T00:20:14Z
updated: 2026-11-07T02:20:30Z
verify: [manual]
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/chromium-lab-browser]
---

# Native browser UI and CEF boundary

## Rule

Version 0.2.0. Implement the user-accepted Meari management-window slice first.
Authored application and tests are Rust; construct typed Meari IR directly, without
TSX, HTML application source, JavaScript or embedded script strings. Pin Meari
1482b746ebc0d867f25d17d01c48c12df888ca6c (0.2.0, MIT OR Apache-2.0), Rust 1.99.1.
CEF is required for target content, but an absent native adapter is NotReady.
The initial management-only window contains no simulated browser. The Windows
native qualification extension below embeds the actual CEF-owned child.

UI-01: A real Meari window presents named address input, synthetic profile and
tabs, diagnostics, scenario and audit status. Keyboard and AccessKit actions use
the same native controls. Address editing does not imply navigation delivery.
UI-02: All permissions default deny; no target filesystem/admin/bearer/command
bridge. Only explicitly configured owned HTTPS origins are candidates for future
navigation. URL parsing belongs to the standard URL crate, not hand-built parsing.
UI-03: Consumer-owned windowed CEF boundary tracks generation and explicit
Detached/Attaching/Attached/Closing/Closed states. Unknown sandbox or missing
focus, IME, keyboard or accessibility support prevents attachment readiness.
Resize accepts bounded positive dimensions and finite DPI; stale callbacks cannot
resize/focus/close a replacement. Cancellation revokes generation immediately;
native cleanup completion is a separate receipt, never inferred from cancellation.
UI-04: No --no-sandbox, site-security disabling, external-window docking or video
surface masquerade. Meari scoped parent leases cannot become stored raw handles.
Actual CEF startup requires independently verified native artifacts/bootstrap and
sandbox on the current OS. Do not create a safe-looking boolean attestation API.
UI-05: Native UI tests and library lifecycle tests do not establish CEF support,
OS assistive-technology qualification, Docker product e2e or server ALLOW.
Keep diagnostics/scenario/audit NotConnected and CEF NotReady until measured.
No scoring, DRM-based rule, motor richness or speed requirement is added.

## Why

Blue review separates trusted controls from untrusted target processes. Red
review identifies fabricated attach/sandbox receipts as a P0: no production CEF
adapter is exported in this slice. SRE review requires explicit cancellation and
cleanup states. Privacy review requires synthetic profiles only, with no personal
browser files/cookies. Evaluator review requires visible non-readiness and distinct
unit/native/Docker evidence. These findings are reflected in UI-01 through UI-05.

The generic Meari native-child API is a candidate seam; reserve_video is excluded.
CEF Windows M138+ bootstrap/client DLL sandbox, macOS helper order and Linux
namespace qualification remain required before target browsing can be enabled.

## Consumer-owned windowed CEF contract

This is the native adapter contract. The original portable guard deliberately
has no Attached transition or cleanup-ACK setter; it cannot attest CEF readiness.
The separate Windows desktop adapter below implements native ownership and still
does not grant production readiness from a successful creation request.

| Operation | Required boundary and receipt |
| --- | --- |
| Create | NativeChildRegistrar callback borrows NativeScopedParent only during CEF child creation; retain NativeParentLease rather than an extracted raw parent handle. Bind profile, tab, owner, host epoch and creation transaction. CEF on_after_created confirms the same transaction before Attached. Cancellation before that callback closes the late child without exposing it. |
| Resize/DPI | Recheck parent lease/current host geometry and UI thread, validate positive bounded physical size and finite scale; request child resize. Requested geometry is distinct from observed OS client bounds. No fake video node or external-window repositioning. |
| Focus/keyboard | Native child HWND/NSView/X11 integration owns focus and tab traversal. Management shortcuts must not steal composing input. Restore focus to a live named control on close; retain explicit lost/unknown focus. OS keyboard events are not human attestation. |
| IME | Preserve enabled/preedit/commit/cancel composition, Unicode and candidate placement through native APIs. Composing key events must not activate tab/navigation commands. Unit-injected IME events do not qualify OS candidate windows. |
| Accessibility | Expose the real CEF accessible subtree under the native child, named controls/actions and traversal. Missing AX means unsupported/unqualified, never replace page accessibility with screenshots. Native AT and zoom require actual OS pilots. |
| Close/cancel | Revoke generation before input; request CEF close on its UI thread; retain child/parent resources until on_before_close and actual host cleanup. Crash, forced OS destroy, callback loss and deadline expiry leave explicit incomplete cleanup. No ack derived from command acceptance. |
| Permissions/navigation | Exact configured owned origins; deny filesystem, capture, downloads, popup, external protocol, admin/credential/native bridge by default. Target callbacks never possess authority credentials or arbitrary local IPC. |
| Startup | Current native artifact identity and supported bootstrap/helper are prerequisites. Keep no_sandbox=0; missing or unverified sandbox fails closed. A non-null pointer, Cargo feature or successful CEF initialize is not independent sandbox proof. |

The local Windows x64 source closure check at the pinned cef-rs revision passed
with sandbox enabled. It resolved CEF156.0.15/Chromium156.0.8078.80. No authored
CEF client DLL or child attachment was delivered by the initial ead2dc6
management-only slice. The subsequent Windows prototype below adds those paths
inside a separately scoped native FFI boundary; cross-OS support remains unrun.

## Verification

### Windows native continuation, 2026-11-07

The user explicitly requested actual bootstrap/child implementation. Two desktop
boundary DLLs under crates/browser/native isolate OS/CEF FFI from the existing
forbid-unsafe core. A minimal bootstrap client DLL forwards the upstream-owned
sandbox pointer to CefExecuteProcess before any UI/native-host loading; only the
browser process subsequently loads the fixed sibling Meari desktop DLL. This
avoids importing the Meari GUI dependency closure into sandboxed subprocesses.
Upstream bootstrapc.exe is renamed to the matching client DLL stem. No authored
C/C++/JS/TS bootstrap is permitted. No subprocess_path or no_sandbox override.

Unsafe is denied by default in these boundary crates, with local reviewed
allowances only for exported ABI entrypoints, fixed-path DLL loading/call and
Win32 geometry/token/handle operations. Exported functions catch panics and never
unwind across C. Native parent handles are borrowed inside registrar/lease
callbacks, never cached as parent authority. CEF state owns its child Browser;
on_before_close is the cleanup receipt. Child creation permits about:blank or one
exact static data URI emitted by Meari's Rust markup builder from typed Rust IR.
This is untrusted synthetic test content, not an HTML application shell. No
authored HTML/JS/TS source or script string implements the UI or fixture behavior.
JavaScript is disabled for this bounded fixture; form editing is native Chromium.
All other resource requests, popups, file dialogs and permissions are denied.
Extensions/component updates/background networking are disabled for the pilot;
OS-enforced network egress isolation is not established by those switches.
Production
navigation remains disabled regardless of startup/attach success.

Windows CEF owns its supported multi-threaded UI loop; Meari owns the main winit
loop. The thread-safe asynchronous CEF creation API copies WindowInfo during the
scoped parent visit. The native-child transaction retains NativeParentLease and
the Meari lifecycle retains the parent until actual CEF cleanup. No application
raw parent handle is cached. Late creation after revocation immediately closes
on CEF's UI thread and never becomes a ready target. Native calls recheck the
lease; parent WS_CLIPCHILDREN prevents overlapping paint feedback. Resize only
changes the owned child when bounds/DPI actually change. Focus and close use
bounded CEF UI tasks. DoClose returns custom handling and queues destruction of
the internal CEF child on its own creating thread; Meari parent release follows
on_before_close, avoiding the default top-level-close dependency cycle.

The close deadline is independently woken, cancellable and joined on every
return/unwind before DLL unloading. An incomplete cleanup deadline or panic is
fail-stop with nonzero process status, never a cleanup receipt. The fixed sibling
desktop DLL is loaded with DLL_LOAD_DIR/DEFAULT_DIRS search, excluding CWD search.

Native process inspection opens only the pilot's descendant IDs and rechecks the
exact private executable image before querying tokens or bounded command-line
data. WDK ProcessCommandLineInformation is version-dependent: failure is recorded,
never an inferred role. Retain only process type, not command lines/profile paths.
Validate UNICODE_STRING allocation bounds/alignment before dereferencing. Join
renderer roles with actual restricted-token and integrity-RID observations. These
properties are limited sandbox evidence, not a proof of complete sandbox policy,
file-access denial, every utility sandbox or escape resistance. Required normal
browsing remains unready on missing proof. Authenticode NotSigned is a real
distribution gate; local measured hashes do not constitute publisher signatures.

Native acceptance: actual upstream bootstrap -> Rust DLL -> Meari prepared
window -> registered CEF windowed child with observed OS parent, native renderer
process/token inspection, resize bounds/DPI, focus and normal child-first close.
Missing sandbox proof fails qualification; no arbitrary target is enabled.
IME/AX must be reported separately: an AX value action on the generated field is
not OS IME composition/candidate placement, keyboard delivery or screen-reader
qualification. The observed keyboard injection without value readback remains
unqualified. Failure, timeout, missing callback and
unsupported OS remain explicit. All raw native evidence stays private.

The bootstrap contract follows the
[official CEF sandbox setup](https://chromiumembedded.github.io/cef/sandbox_setup)
and the pinned distribution's cef_sandbox_win.h/cef_life_span_handler.h. The
current console ABI has four arguments, including an opaque version-info pointer.
DoClose/OnBeforeClose ownership is checked against that actual header. Native
file-access denial and full utility-process sandbox policy are not claimed.
Synthetic profiles generated by the pilots remain private runtime cache. An
automatic approval review rejected their bulk recursive deletion with only
`blocked by policy` as its stated reason. No alternative deletion was attempted.
Do not publish or inspect these generated profiles as ordinary evidence content.

Run meaningful lifecycle/policy unit tests, actual Meari document keyboard,
AccessKit/input/resize tests, then launch the native window with synthetic data.
Record actual build and run failures. Screen-reader, IME composition, native DPI,
CEF attach/sandbox/renderer crash and six native tuples remain unqualified until
their specific pilots run. Docker-only backend/product e2e remains authoritative.

### Native input and sandbox probes, follow-up contract

The opt-in --native-korean-ime pilot may select an already installed Korean
layout only for CEF's calling UI thread (ActivateKeyboardLayout flags0) and open
that owned child's IME context. It must not install a layout, set process/global
layout flags, change security/privacy settings or inject committed text. Observe
physical key delivery separately from native Unicode value actions. Read only
input language IDs, context/open state, composition byte length and candidate-form
coordinates, never composition text. Candidate-form geometry alone does not prove
a visible candidate window. Bound retained transitions to64 and release contexts.

The bootstrap may register a minimal Rust renderer handler for a bounded sandbox
probe after CefExecuteProcess starts the renderer. Browser creation extra_info
provides only the current owner PID; the renderer derives a fixed synthetic
canary filename in its own executable directory, attempts a read and create_new
of a fixed own-PID probe file, and sends only PID/error codes via CEF IPC. The
browser accepts only the exact message/version, renderer source and three integer
arguments, bounded to32 unique PIDs. Correlate with owned OS renderer tokens;
ERROR_ACCESS_DENIED(5) is denial, other failures or missing IPC remain inconclusive.
Never read personal paths or enable target navigation. These two checks do not
qualify utility policy, network isolation or sandbox escape resistance.

Native key/focus instrumentation retains counters only, not keystroke contents.
An independent hard deadline must survive a blocked close wake. Both worker
threads are joined before native DLL unload; an incomplete deadline is fail-stop.
The user subsequently explicitly authorized deletion of generated private
synthetic-profile directories. Check resolved paths remain direct children of
the private bundle, reject reparse points, and stop owned processes before deletion.

References: [ActivateKeyboardLayout](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-activatekeyboardlayout),
[IMM functions](https://learn.microsoft.com/en-us/windows/win32/intl/input-method-manager-functions),
and the pinned CEF RenderProcessHandler/Client headers and sandbox setup above.

The opt-in pilot uses F8 only on a focused owned CEF input context to open Korean
native conversion mode, and F9 to request real IME conversion/candidate opening.
No synthesized Hanja event, committed-string injection or custom candidate UI is
used. Record API acceptance separately from actual candidate display and geometry.
The latest second deletion attempt after explicit user approval was again rejected
by automatic approval review;32 generated profiles remain private and no bypass
was attempted. evidence:root/20261107021004 records the current measured scope.

OSK follow-up on unchanged a924603 source: the built-in Korean on-screen keyboard
exposed a Hanja key. Its click had no confirmed key-delivery/candidate receipt.
Candidate qualification remains open. The owned CEF pilot exited0. OSK UI-close
had no confirmed effect; stopping only its introduced PID/start-time matched
process returned access denied. No elevation/security bypass was attempted; the
introduced OSK window remains for manual close.32 generated profiles now remain.
