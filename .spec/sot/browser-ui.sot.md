---
id: sot:root/browser-ui
title: Native browser UI and CEF boundary
status: active
created: 2026-11-07T00:20:14Z
updated: 2026-11-07T05:36:25Z
verify: [manual]
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/chromium-lab-browser]
---

# Native browser UI and CEF boundary

## Rule

Version 0.5.0. The browser-first shell and research contract below supersede the initial management dashboard.
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
bridge. Future external passive navigation follows the scoped research admission contract in sot:root/chromium-lab-browser. Active experiments remain owned/authorized. The current fixture pilot still denies external navigation. URL parsing belongs to the standard URL crate, not hand-built parsing.
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

### Measured Hanja native input pilot

The user explicitly authorized Rust native replay of a measured physical key:
VK_HANJA0x19, message scan0xF1, extended=true. --native-measured-hanja together
with --native-korean-ime permits F10 to enqueue one OS SendInput down/up pair
(up to4 attempts). Retain the supplied virtual key and scan with EXTENDEDKEY;
do not use UNICODE or SCANCODE mode (SCANCODE would ignore the supplied VK).
The user's repeat=true is not copied into a held/repeated key. Reject input unless
on CEF UI thread, current foreground belongs to this process, focus is in its
CEF child and no Shift/Ctrl/Alt/Windows modifier is held. Never steal foreground,
elevate, bypass UIPI, install global hooks or send to another app. Foreground
checks do not make OS input routing atomic against concurrent user focus changes;
use a cooperative local pilot and keep this path out of production navigation.

Record inserted event count separately from CEF receipt and actual native IME
candidate visibility, screen placement, selected value and cancellation. API
success is not a candidate receipt. Only synthetic field text and bounded native
metadata may be retained. Sandbox bootstrap, deny policy and BrowserReady=false
remain required. This explicitly requested native pilot supersedes the earlier
no-raw-helper investigation constraint for this measured key only.
## Browser-first UI contract UI-06 through UI-12

UI-06: User-selected layout is a compact browser tab strip and navigation/address
bar above a real CEF page, with an initially collapsed inspector dockable right
or bottom. Replace the former large headings/diagnostic paragraphs and fixed560
DIP content offset. The page surface fills remaining client area without decorative
gutters. Inspector sections are not browser tabs. Only actual live target tabs
are shown as target tabs; unsupported new/close/history/navigation controls are
explicitly disabled instead of pretending they work.

UI-07: Native allocation and Meari chrome use one shared layout model and constants:
default strip34 DIP + toolbar50 DIP, content top84 DIP, no footer reservation.
Right dock width=min(32% client width,340 DIP); bottom height=min(40% client
height,260 DIP). Allocate from actual native client dimensions and DPI, with
rounding at the physical boundary. Recheck the parent lease, resize the existing
CEF child on dock/viewport/DPI change, and verify actual OS child bounds. No new
browser, page reload, reparenting or zoom/scroll/history reset on inspector toggle.
A later renderer-layout receipt must cover text enlargement rather than claiming
fixed84 DIP works at every accessibility zoom. Native OS caption is separate.

UI-08: The inspector's initial categories are local engine observations, page
exposure hypotheses/experiments, and optional server receipts. Tabs keep accessible
names and keyboard state; dock controls expose hidden/right/bottom states. For a
nonintegrated site, server evidence is not_integrated/unavailable, not a failed
page or an implied ALLOW. Unknown/missing observers show not connected/unsupported,
not clean zero-event counters or fake classification. Status is concise and cannot
consume most of the page. Untrusted content cannot activate native operations.

UI-09: CEF/Chromium renders target HTML/CSS/JS/fonts/media. Meari cannot render,
rebuild or fake the target page. The native pilot still uses a script-free local
typed-Rust-IR fixture and denies external content; normal Chrome-like browsing is
a separate admitted implementation slice. Do not promote its fixture restrictions
into normal research mode or assert Chrome equivalence from this UI preview.
Rendering fidelity, exact-version cohorts and capability/codec/service differences
are governed by RSRCH-15 in sot:root/chromium-lab-browser.

UI-10: Address editing, actual navigation request, accepted destination, committed
engine URL/loading/error and security/admission status are separate states.
Programmatic/redirect changes synchronize the address only from an actual engine
receipt; editing text alone is not navigation. The first compact shell may retain
navigation lock and one real fixture tab while multi-tab/session/observer wiring
is incomplete. Feature status belongs in the inspector and capability manifest.

UI-11: Preserve focused controls, real page AX subtree and IME ownership while
docking. Do not capture composing keys as shortcuts. Default-scale metrics target
<=88 DIP top chrome and >=89% content height at1280x800 client, inspector closed.
Allow accessibility text enlargement to grow controls. Prove actual dock toggles,
physical child bounds within1 pixel, focus/keyboard/zoom/screen-reader behavior
per supported DPI/OS tuple. A model test is not a native layout/AX receipt.

UI-12: Read-only panel synthesis used blue/red, privacy/SRE and evaluator views.
P0 engine rendering confused with shell/UI imitation ->06/09 and RSRCH-15.
P0 privileged debug knowledge promoted to site detection ->08/10 and RSRCH-03.
P0 sandbox weakened for page loading ->02/09 and RSRCH-05/07. P1 inspector overlap,
fixed header clipping or new-child reload ->07/11. P1 misleading disabled controls
or not_integrated labels ->06/08/10. These findings are requirements, not new
measured coverage. Shared project conventions remain in sot:root/rust-migration
and AGENTS.md: bounded-context modules, Rust naming/testing, no duplicate engines,
reviewed narrow unsafe boundary and attributed serial git operations.

## Undecorated Windows host and SVG icons

UI-13: The Windows pilot removes OS caption and thick-frame decorations from its
own leased Meari parent. Keep actual CEF content, client DPI allocation and close
coordinator. Draw accessible named minimize, maximize/restore and close buttons
in the existing34-DIP strip. Pointer, keyboard and AX use the same local activation
paths; close requests coordinated child/host cleanup, never DestroyWindow alone.
Only blank title-strip space may initiate OS dragging. Do not claim edge resize,
Snap layouts, text scaling, high-DPI/AT qualification without native evidence.
Standalone controls preview disables native commands when no host is attached.

UI-14: Add a reusable Rust-authored SVG icon module with a closed symbol enum and
constant typed Meari IR path geometry. No font-glyph dependency, remote assets,
script, arbitrary SVG parsing or authored HTML shell. Decorative SVG is hidden
from AX; containing buttons retain meaningful names and disabled state. Verify
actual vector rendering; a parsed document alone is not a pixel receipt.

UI-15: Native SDK/DLL expansion is governed by sot:root/native-embedding and
plan:root/20261107041001. The process-owning experimental browser DLL is explicitly
not the SDK ABI. No SDK crates, automation-blocking policy or changed scoring
are delivered by this UI slice. Existing sandbox-required and navigation locks
remain in force. P0 unsafe DLL reuse and false sandbox claims become SDK-03/07;
P1 lost window controls/AX and icon glyph failures become UI-13/14.


UI-16: User-expanded resize scope supersedes UI-07's zero-gutter allocation at the
outermost perimeter: reserve an unobtrusive5-DIP app-owned hit strip on left,
right and bottom so the CEF child cannot swallow resize input. Top edge shares
existing chrome. Eight edge/corner directions use corresponding resize cursors;
interior inputs/buttons retain native text/pointer cursors. The strip is rendered
by the app, not an OS caption/frame. Page allocation uses the same bounded DPI
constant; default1280x800 content height target becomes >=88.5% (711 pixels),
with84-DIP header unchanged. Right/bottom inspector does not cover outer hit strips.
Resize clamps to DPI-scaled640x480 DIP minimum and16384 physical maximum; opposite edges stay
fixed. Maximize disables edge resize. Capture is local to the current owned
window, releases on button-up/focus loss and never uses global hooks. Measure
actual geometry changes, CEF resize readbacks and cursor observations separately.


UI-16 critique reflected: maximize must explicitly override the resize-strip CSS
cursor to arrow; merely suppressing native resize is insufficient. Minimum size
scales with DPI so bottom/right docking cannot produce negative CEF bounds at
higher DPI. Cancel active capture on DPI transition. Native evidence remains96DPI;
unit checks cover48..768 DPI and every dock at minimum size. Post-review max-cursor
and high-DPI branches are built/unit-tested, not separately native-qualified.


## Real newtab and individually admitted internal diagnostics

UI-17: Default attached content becomes the actual CEF-rendered app-owned
humanymous://newtab/ document, not a Meari overlay or screenshot. Construct bound
Meari typed IR and serialize through upstream html5ever; no authored HTML/JS/TS
application source, parser or hand-built HTML serializer. The script-free page
contains a real GET address form, current-tab diagnostic links and honest local/
not-qualified status. No fake recents/history, remote assets, search tracking or
native bridge. Preserve the old generated input fixture under explicit native
qualification flags; no Hanja replay is required by this slice.

UI-18: Register the owner scheme in every CEF process through shared Rust code,
including bootstrap renderer App and desktop browser App. Only the exact owner
host/path returns bounded immutable generated bytes. No URL-to-filesystem mapping,
CORS/fetch/CSP bypass or privileged native handler. Newtab response CSP forbids
script/connect/object/frame resources, allows only its required inline style and
same-owner form action. Form/relative-link actions are exact bounded routes,
accepted only from the current main-frame newtab with a user gesture.

UI-19: Native address Go/Enter and all page links use one exact-route policy.
Preflight BEFORE browser-initiated LoadURL is mandatory: Chromium handles some
quit/restart/about actions before navigation throttles. Also guard CEF browse,
open-from-tab, popup and resource callbacks. No wildcard chrome:// navigation.
Candidate read-only pages are version/gpu/credits/license/sandbox; admit only the
individually verified subset of this pinned CEF156.0.15/Chromium156.0.8078.80,
Chrome-runtime/Alloy-style Windows tuple. chrome-urls/settings/flags/extensions/
inspect/net-export/crash/hang/quit/restart and unknown variants remain denied.
chrome-urls is not read-only (it exposes debug-page preference mutation).

UI-20: Internal chrome://resources/theme and same-page assets may load only for
an admitted internal page; asset permission never grants top-level navigation.
No external HTTP(S), file/data/javascript navigation or remote subresource in this
slice. Preserve media/file/popup/admin/bridge denial and sandbox-required bootstrap.
Enable upstream builtin JavaScript required by diagnostic WebUI; this does not
authorize authored app scripts or injecting script strings. Newtab remains CSP
script-free. Do not replace or imitate built-in chrome pages with application HTML.

UI-21: Requested/accepted/committed/load-error/content identity are separate.
Update address only from actual current main-frame receipt, without promoting
an error/fallback page to support. Native Enter applies only to focused address,
never composing/repeated/IME-confirm Enter. Form submission inside actual CEF uses
browser native semantics. Preserve one real tab; '+' stays disabled until actual
multi-tab creation. Home/newtab return is navigation, not another invented tab.

UI-22: Bounded validation probe visits only the fixed harmless candidate list;
never execute crash/hang/settings probes. Record structural/content identity,
source byte budget, observed error and actual receipt; budget exhaustion is not
unsupported. Diagnostic source/AX/screenshots may reveal paths/device data and
remain private or transient; public evidence uses safe route/structural receipts.
A sandbox diagnostics page is not full OS sandbox qualification. BrowserReady,
external browsing and unmeasured platform/IME/AT/Chrome parity stay unqualified.


### UI-23 — exact local navigation receipt contract

Implemented in `internal_urls`, `newtab`, `ui_navigation` and native
`navigation`/`local_page`/`browser_controls`. The host preflights a typed Route
before posting LoadURL. Canonical routes and slashless spelling only; no query,
fragment, credentials, port, path alias, external URL or arbitrary scheme. Owner
GET form and relative links are translated only from current main-frame newtab
with a gesture. Allprocess custom-scheme registration uses STANDARD, SECURE and
DISPLAY_ISOLATED; no CORS/FETCH/CSP-bypass option. Immutable UTF-8 HTML uses
upstream html5ever serialization of typed Meari DOM. CSP forbids script, connect,
object, frame and image; inline styles and same-owner form are the only allowances.

Receipt schema v1: `generation:u64`, `route:fixed canonical string`,
`phase:creation_requested|request_accepted|renderer_request_accepted|committed|load_end|load_error|content_observed|source_budget_exceeded|probe_timeout`,
`code:i32` (CEF error or load status, otherwise0), `bytes:usize`, `identity:bool`.
64 navigation generations,128 receipts,4096 UI activation requests and16 intents
per poll bound the pilot. Source content is transient; only structural identity
is retained. The initial4MiB bound exceeded the actual credits document
(5,225,290 UTF-8 bytes); the accepted bound is8MiB. Preserve that first budget
exhaustion, never relabel it unsupported or silently replace its receipt.
Request acceptance never means commit or content support. Unrecognized routes
produce a fixed safe native notice and no privileged navigation. Composition,
commit confirmation, repeats and unfocused Enter cannot navigate. External
BrowserReady remains false.

The root-owned shared Cargo.lock update for optional html5ever0.40.1 and pinned
meari-dom was requested and accepted by root on2026-11-07; the browser owner
edits only its package manifest. Future SDK/ABI/framework/CLI crates require
separate root integration; this change adds no SDK workspace member.


### UI-24 — applied Windows support matrix and native frontier

State: accepted, implemented and bounded-native-verified for this exact tuple.
Evidence:root/20261107053625 supersedes the earlier internal-URLs-not-run claim.
`humanymous://newtab/` and chrome version/gpu/credits/license/sandbox canonical
root paths are admitted (slashless spelling is normalized by typed host dispatch).
Everything else, including chrome newtab/chrome-urls/settings/flags/extensions/
inspect/net-export/debug/destructive URLs, remains policy-denied and was not
executed. Subresource grants for chrome resources/theme/same-page host do not
grant navigation. No universal Chromium URL compatibility is claimed. Version
fields, GPU dynamic feature status, credits, license and sandbox status were
observed through native AX/visual state, with private details not exported.

A host chrome pointer press must transfer Windows keyboard focus to the live
scoped parent, verify GetFocus and then receive native address editing. DOM focus
alone proved insufficient with an embedded CEF child; after the scoped SetFocus
fix, native Ctrl+A/text/Enter committed chrome license, reload committed the same
route, home returned to newtab and 1116-to988-pixel resize retained actual CEF.
CEF page form Enter separately opened sandbox. Page AX was readable; native shell
AX automatic click bounds were absent. Coordinate clicks do not qualify AT.

Source inspection also requires explicit download refusal: CanDownload=false,
OnBeforeDownload handled=true with no continuation (release cancels per pinned
CEF contract), and OnDownloadUpdated.cancel. This final delta is built/tested/
Clippy-clean and its exact artifact passes a12-second safe-page/cleanup probe;
the interactive export callback path is not exercised. Existing file dialogs,
permissions and popup refusal remain. This is no generic bridge or SDK ABI.

The source visitor first bounds8Mi UTF-16 code units before allocation, then8MiB
UTF-8 structural inspection. If rejected before conversion, bytes=0 means no UTF-8
byte count observed. Native metadata/source remains transient; raw page data is
never placed in public receipts. Newtab has explicit UTF-8 charset metadata.

BrowserReady=false remains a hard release/external-navigation gate. Observed
renderer restricted tokens and canary read/create ERROR_ACCESS_DENIED are a
bounded premise, not full sandbox attestation. The sandbox WebUI reports Network
Service Not Sandboxed and RendererAppContainer disabled; independent network,
utility/GPU/job/mitigation policy and signing checks remain. A transient GPU
exit78 recovered enough for populated hardware-acceleration diagnostics; recovery
is not full qualification. A version logo asset was broken in an earlier native
view; exact Chrome assets/parity remains unqualified. Host font fallback was used.
No OS screen-reader, new-layout Korean IME/candidate, high-DPI/multi-monitor,
external browsing/history/multi-tab or distribution completion follows.

Primary pinned source review: CEF cf671c0 tests/ceftests/webui_unittest.cc and
include/cef_download_handler.h; Chromium156.0.8078.80 chrome_urls_handler.cc,
browser_about_handler.cc and ui/navigator/browser_navigator.cc. Source registration,
policy admission, observed page content and full product support remain distinct.
