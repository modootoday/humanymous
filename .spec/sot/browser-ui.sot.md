---
id: sot:root/browser-ui
title: Native browser UI and CEF boundary
status: active
created: 2026-11-07T00:20:14Z
updated: 2026-11-07T08:01:06Z
verify: [manual]
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/chromium-lab-browser]
---

# Native browser UI and CEF boundary

UI-34: a typed history/navigation ticket remains authoritative across CEF's
OnOpenURLFromTab then OnBeforeBrowse callback chain. Only a live pending exact
target in the main frame with CURRENT_TAB disposition may continue without a new
ticket. Renderer gestures without such authority retain separate guarded intent
dispatch; foreign routes/frames/dispositions and stopped/completed/revoked/closed
work never gain continuation authority. Record bounded open_continued receipts;
they establish callback admission only, never commit/content or BrowserReady.
Do not ignore ERR_ABORTED or infer callback generation from route equality alone.
Same-route stale native callbacks remain an explicit attribution limitation until
an independently verified engine identity seam resolves them.
This includes start/end/error callbacks and open_continued: route equality does
not identify a generation. The source reproducer begins A(r), then B(r), and
delivers A's route-only start/error; current code attributes them to B. Captured
task/source-visitor ticket rejection does not discharge this adapter limitation.

## Rule

Version 0.6.1. The browser-first shell and research contract below supersede the initial management dashboard.
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


## Accepted execution policy linkage

Execution policy is owned by sot:root/program-roadmap and implemented by
sot:root/delivery. Browser UI version0.5.1 records only its boundary, not another
scheduler/concurrency/cache/artifact policy.

Local checks cover source/fmt and resource-bounded pure URL-policy/lifecycle/
geometry units or scoped Clippy under the central Cargo limit. Meari native-ui
compilation is not assumed lightweight. Actual CEF initialization, HWND child
attachment, process/token/canary/IPC and shutdown probes use the approved hosted
runtime path. Interactive desktop input, Korean IME candidates, OS AT/UIA bounds,
physical GPU, mixed DPI/multi-monitor and actual Chrome comparison require direct
suitable-environment evidence; a hosted label or unsupported/skipped test is not
qualification. Full sandbox/BrowserReady/external-navigation gates stay closed
until their required premises are established.

The initial newtab checkpoint had not_run hosted qualification under that policy.
Later exact-source bounded Windows results are recorded in
plan:root/20261107071648 and evidence:root/20261107073932; they do not qualify
GUI/IME/AT, full sandbox policy or all supported tuples.
Earlier source-bound local receipts remain historical; they do not qualify new
sources or hosted jobs. No runtime/build/dispatch was started for this policy
record. Public collection and missing private-original handling defer to the
central delivery contract; browser diagnostic paths/device data stay out of
public artifacts. Required native gates are not removed by path heuristics.


## Navigation lifecycle and probe closure contract

UI-25 (accepted implementation slice; execution evidence is source-bound): one pure Rust
navigation authority owns bounded64 generation tickets, current request,
observed commit and explicit terminal outcome. A ticket binds generation and an
exact typed local route. New requests supersede pending tickets; revoke is
irreversible and precedes child cleanup. A rejected UI task, absent main frame,
load failure, content mismatch, source budget exhaustion and probe timeout are
explicit failures, not accepted requests left pending. Stale captured task/source
tickets, closed/revoked callbacks and post-terminal completions cannot mutate a
new transaction. Last actually observed committed address is separate from a
newly requested/failed address. Native wrappers own OS/CEF calls and serialization;
they must not fork the pure transition rules.

UI-26: --native-internal-probe requires ordered newtab, five admitted diagnostics
and return-to-newtab observations. Every step has its own captured ticket and
bounded deadline; the initial document is also bounded. Independent terminal
failures are collected across all seven steps; malformed order, reused/superseded
tickets abort rather than manufacturing a pass. Process cleanup success alone
cannot satisfy the probe: requested but incomplete/failed probe returns nonzero
after retaining the report and cleanup receipts. Probe completion/result/count and
receipt truncation are explicit report fields. Receipt capacity remains bounded;
dropped observations are counted, never silently treated as full provenance.

UI-27: address length/control bounds precede URL parsing and query decoding.
Owner actions accept exactly one address field, without allocating an arbitrary
query-pair vector. IME disable/focus loss cancels retained composition with a
confirmation barrier so canceled composition neither navigates nor permanently
prevents later deliberate Enter. Native keyboard/IME/AX proof remains a separate
hosted/suitable-environment gate.

Review limitations: CEF LoadHandler does not supply the host request ticket.
Route-matched load-start/end callbacks cannot fully distinguish an older same-URL
navigation from a newer one. Captured UI tasks/source visitors are generation
checked; this is not unique engine-navigation attribution or BrowserReady.
Full sandbox, external browsing, IME/AT/GPU qualification remain unchanged/closed.
All new source/results require their own source-bound hosted execution; previous
586ff95 native receipts cannot validate this slice.


Navigation report revision2 is additive to the prior receipt payload: fields
navigation_schema=2, navigation_receipts_dropped:u64, internal_probe_requested:bool,
internal_probe_finished:bool, internal_probe_observed:usize and
internal_probe_passed:bool. finished includes abort and is not success; passed
requires all7 positive ordered observations and zero dropped receipts. New terminal
receipt phases task_rejected and frame_unavailable retain code0/bytes0/identityfalse.
A required-probe runner must require schema2/requested/finished/observed7/passed,
zero receipt loss and real cleanup receipts; missing fields from an older report
are unknown, not implicit pass. No required gate may use exit0 alone.


OnBeforeBrowse only continues the exact pending host target. An out-of-order
browser callback cannot create a replacement renderer transaction. Renderer
links must carry a current-main-frame admitted source and user gesture; they are
canceled and redispatched through the typed host queue. Programmatic cross-route
redirects and implicit renderer reload without a host ticket stay denied. This
centralizes transaction ownership while retaining the same-URL callback limit.


Probe qualification failure is distinct from uncertain cleanup. The native host
returns a clean-exit token only after actual child before-close and parent-closed
receipts. Desktop then shuts CEF down before returning a failed probe result.
Uncertain cleanup retains the existing process-owning fail-stop boundary; this
is not an embeddable SDK ABI. Scheme-factory registration failure occurs before
browser creation and shuts initialized CEF down before reporting the error.


The measured fixed local tuple previously returned main-frame status200 on every
admitted route. Revision2 requires that status before positive source inspection;
other statuses terminate as load_status_rejected (code=observed status), distinct
from CEF load_error codes. Terminal failures project a fixed safe native notice;
raw URL/error text is not echoed. This stricter tuple gate needs hosted replay.


Revision2 also records superseded and revoked for an outstanding old ticket,
without attributing those events to its replacement. Request budget refusal
leaves the prior transaction intact; supersession is recorded only after a new
request is accepted. Repeated revoke is idempotent. Receipt loss remains explicit
and disqualifies the requested probe even when its in-memory ledger completed.


Native child revocation atomically revokes navigation authority under the same
owned mutex. Deferred focus/observer tasks, input/IME actions, process-result and
renderer-canary callbacks refuse revoked/closed authority; late diagnostic results
are not appended after revocation. Close itself remains permitted while stopping
until actual before-close, and does not become a focus/observer grant. No global
hooks or third-party window/process inspection is introduced.


UI-28 (accepted source follow-up; execution evidence is source-bound): owner-scheme
registration records the actual add_custom_scheme return in process-local atomic
state. Unknown/missing/rejected registration is not success. Desktop factory
registration refuses to proceed unless its browser App observed acceptance;
initialized CEF is shut down before reporting this pre-browser failure. Renderer
canary work also refuses to run when its own App lacks accepted registration.
No filesystem or canary denial is inferred from the absence of a renderer report.
This is a process-local prerequisite, not all-process registration attestation,
full sandbox qualification or a new BrowserReady value. Existing report/schema2
fields do not retroactively prove UI-28. Old source c4b58be remains independently
bound; this native delta requires its own compile/probe receipts. The missing-
registration fault case remains separate from a normal accepted registration run.


UI-29 (accepted verification-source slice): the hosted consumer must independently
validate the launched OS PID, exit classification, report revision, current
request flags and actual ordered commit/load/content receipts, not only the
producer's passed boolean.
For each successful route, its generation and route must have a matching request
receipt before commit: creation_requested for the initial page, request_accepted
for later pages. A positive report must explicitly deny cancellation.
Positive evidence requires at least one renderer
canary joined by PID to an owned restricted/Untrusted renderer OS observation;
missing joins are unknown/failure. Canceled-probe negative requires exit78,
cancellation requested, incomplete probe and actual child/parent cleanup. Public
summaries contain only fixed case labels/counts/flags/PID, never raw report bodies,
paths, device text or captured stderr. Protocol tests with synthetic reports are
not native browser executions. Reuse the shared test-support process supervisor;
new dependency edges wait for root's approved graph window.

The sole expected hosted summary/count contract is
crates/browser/native/desktop/pilot-contract.json. Both native verification and
CI receipt accounting consume this owned data. Summary schema1 binds two exact
test names: positive has seven observed routes and one to32 joined renderer
probes; creation-time cancellation has zero observed routes and zero joined
probes. Host cancellation prevents subsequent control/probe polling. Reports
with one to six observed routes are not this cancellation case. Raw reports
retain separate navigation and file-probe schemas; summary schema is not a
replacement for their detailed verification or native execution evidence.

Cancellation verification must inspect the report, not manufacture joined0. The
file-probe schema is required for both cases; cancellation requires the explicit
renderer_file_probes array to be empty under the owned contract. Missing arrays,
wrong schema and retained renderer observations fail. Its navigation inventory
must be present, bounded and lossless, start with the initial creation request,
and contain only that initial generation/route and recognized callbacks. A stale
multi-route positive inventory cannot become cancellation evidence by changing
the producer's observed/pass flags. Initial CEF callbacks may race the close
request; their runtime behavior remains unqualified and must not be masked by
synthetic zero counts. Protocol execution is source-bound; passing synthetic
report regressions does not prove actual desktop or accessibility behavior.

The same owned contract lists protocol_preflight: its exact selector and four
synthetic report test names, separate from the two actual process case names.
These tests do not initialize CEF or start processes; compiling their Windows
desktop test binary still requires the native feature/dependency build. The
runner must verify all four exact names and executed count without ignored cases
before separately reporting bootstrap rejection and actual desktop process cases.
Passing protocol fixtures is not CEF execution or native runtime qualification.

UI-30 (accepted source follow-up; execution evidence is source-bound): renderer file
probe schema2 records an independent owned-process OS token observation when each
bounded canary message is received. The browser discovers its own descendants and
checks executable identity; it never opens an arbitrary renderer-supplied PID.
No shared state mutex is held across OS queries. Authority is rechecked before
appending. Missing PID, failed query and absent OS error code remain explicit
unknown observations, never access-denial evidence. The nested token must match
the reported PID and identify a restricted/Untrusted renderer before its claimed
file denials count. This does not attest PID lifetime identity, all processes or
full sandbox policy. Navigation schema2 is unchanged. Historical periodic token
snapshots are not retroactively replaced by this new observation contract.

UI-33 (user-selected tab and shortcut expansion): each tab owns a distinct real
CEF Browser, navigation authority and Meari child lease. IDs never reuse within a
session; at most eight unsettled children and64 lifetime creations are admitted.
Closing or pending-creation children consume capacity until actual before-close.
Switching hides/deactivates the former child, projects the selected tab's actual
address/title/history and resets address IME confirmation state. Stale, hidden or
closing children cannot issue window-level tab/address commands. Close uses the
existing lifecycle release barrier; late-created revoked children close, and
closing the last tab requests coordinated window close. Parent teardown drains
all registered children; one active child's receipt is insufficient.
Private tab receipts retain actual CEF browser IDs after close, per-tab fixed
local route/navigation receipts and loss counts. Distinct UI labels alone do not
prove distinct browsers or retained history. Missing IDs/receipts stay unknown.

Ctrl+T/W/Tab/Shift+Tab/L, Ctrl+R/F5 and Alt+Left/Right share a bounded native intent
queue from both Meari and CEF focus. Ctrl+Shift+Tab means previous tab. Repeated
creation/close accelerators do not create an unbounded queue. Escape, Enter,
Hangul and Hanja remain with the focused input/IME unless separately specified.
Tab UI provides names, selected/hidden/disabled state and distinct close actions.
Inactive tab navigation/history is retained by CEF, not replayed with load_url.
No external navigation, persisted profiles, session restoration or full AT claim
is added. Pure tab/shortcut tests, native scene actions and actual multi-child
switch/close/late-create/parent-close observations qualify separate boundaries.

UI-32 (accepted browser-controls expansion): Back and Forward operate on actual
CEF navigation entries, never an application-invented history. Before dispatch,
the complete bounded entry snapshot must identify one current entry and an
adjacent allowlisted route. Stale snapshots, missing entries, busy transactions,
revocation and unsupported routes deny traversal. The target receives a fresh
navigation ticket before CEF go_back/go_forward; existing request/resource guards
remain authoritative. CEF loading callbacks supply history availability and
loading state; request acceptance alone is not completion. Stop cancels pending
authority before stop_load so delayed callbacks cannot finish stopped work.
Named controls expose disabled state to keyboard and accessibility clients.
Dynamic shell attributes and text commit through the pinned Meari retained-source
API so action metadata, layout and AX state agree with pixels. Direct DOM paint
mutation is insufficient. The shell owns a fail-closed browserReady=false state
binding; mounting that source program does not qualify CEF or grant navigation.
Retained updates must preserve dirty address content, node identity and focus.
The pinned reload contract requires explicit disabled-path migration. Permit its
set delta only for the four standalone host navigation buttons, never composite
roots/triggers/items/close paths or other controls. Preserve widget state and
restore the prior disabled set if preparation/commit fails; unexpected migration
fails the host update. Native AX actions remain the required behavior oracle.
Home, Reload and address submission retain typed local routing and IME guards.
Multi-tab, external navigation, downloads, persistence and Chrome parity remain
separate gates; implemented controls do not imply BrowserReady. Pure snapshot and
document-action tests, native-feature compilation and an actual fresh-bundle
Back/Forward/Reload/Stop pilot are distinct required evidence.

UI-31 (root-accepted test-only bootstrap follow-up): the four fixed security
override cases retain exit78 and unchanged synthetic-profile predicates. Their
direct children use the shared ManagedChild owner for wait/stop/Drop cleanup;
no custom supervisor or production bootstrap logic is introduced. The existing
pilot-contract.json owns the exact case IDs/flags, typed closed observation
fields and synthetic observation tests. Case/profile markers precede assertions;
no PID, path, error message or child output is reflected. Reaped is true only
after observed direct-child status, otherwise null; missing facts never imply
success. Diagnostic observations do not replace actual case/harness receipts,
re-evaluate bootstrap semantics or qualify full sandbox/descendant cleanup.
New producer and synthetic tests require independent native-feature compilation
and execution; older source results are not inherited.
