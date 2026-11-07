---
id: sot:root/browser-ui
title: Native browser UI and CEF boundary
status: active
created: 2026-11-07T00:20:14Z
updated: 2026-11-07T00:32:14Z
verify: [manual]
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/chromium-lab-browser]
---

# Native browser UI and CEF boundary

## Rule

Version 0.1.0. Implement the user-accepted Meari management-window slice first.
Authored application and tests are Rust; construct typed Meari IR directly, without
TSX, HTML application source, JavaScript or embedded script strings. Pin Meari
1482b746ebc0d867f25d17d01c48c12df888ca6c (0.2.0, MIT OR Apache-2.0), Rust 1.99.1.
CEF is required for target content, but an absent native adapter is NotReady.
The management window contains no page frame, preview or simulated browser.

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

This is the next native adapter contract, not an implemented/qualified adapter.
The current guard deliberately has no Attached transition or cleanup-ACK setter.

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
CEF client DLL, interactive child attach, actual sandbox receipt or cross-OS
prototype is delivered by this management-UI slice. No unsafe exception is added.

## Verification

Run meaningful lifecycle/policy unit tests, actual Meari document keyboard,
AccessKit/input/resize tests, then launch the native window with synthetic data.
Record actual build and run failures. Screen-reader, IME composition, native DPI,
CEF attach/sandbox/renderer crash and six native tuples remain unqualified until
their specific pilots run. Docker-only backend/product e2e remains authoritative.
