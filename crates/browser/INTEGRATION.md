# Browser native integration

Owner: misc / rust-browser-cef-native. The Meari management UI is integrated.
Two actual Windows CEF native prototype packages are implemented below; shared
workspace membership/lock integration is root-owned and now present in the shared checkout.
BrowserReady=false; external target browsing is disabled.

## Exact root delta

Root has added `crates/browser/native/bootstrap` and `crates/browser/native/desktop` to
workspace.members and resolved Cargo.lock. Keep both publish=false. The core
humanymous-browser remains unsafe_code=forbid. Native packages explicitly deny
unsafe by default with local reviewed FFI allowances; do not weaken core lints.
They use explicit edition2024, MSRV1.99.1, Apache-2.0, version0.1.0 metadata.
All dependencies are under target.'cfg(windows)'.dependencies. Empty non-Windows
crate compilation is not Linux/macOS native implementation or qualification.

- Both: cef git https://github.com/tauri-apps/cef-rs.git,
  rev224a785c7b41758b51821cbb7395ea57ba493a63,
  default-features=false, features=["sandbox"]. No dox or no-sandbox.
- Bootstrap: libloading="=0.9.0"; cdylib humanymous_cef_bootstrap.dll.
- Desktop: humanymous-browser path="../..", features=["native-ui"];
  meari and meari-native git https://github.com/modootoday/meari.git,
  rev1482b746ebc0d867f25d17d01c48c12df888ca6c;
  meari-native default-features=false, features=["window"];
  winit="=0.31.0-beta.3"; serde(version1, derive), serde_json(version1).
- Desktop windows="=0.62.2", features Wdk_System_Threading,
  Win32_Foundation, Win32_UI_WindowsAndMessaging,
  Win32_UI_Input_KeyboardAndMouse, Win32_UI_Input_Ime, Win32_UI_HiDpi,
  Win32_System_Threading, Win32_System_Diagnostics_ToolHelp,
  Win32_Security, Win32_Security_Authorization, Win32_System_Com,
  Win32_System_ProcessStatus. cdylib+rlib humanymous_cef_desktop.dll.

No shared Cargo files were edited by this lane. Before root integration, a
private detached workspace references the actual native lib.rs paths and actual
core crate; no implementation algorithms are copied into the harness.

## Bootstrap and parent ownership

Use the exact upstream CEF windows64 minimal distribution:
cef_binary_156.0.15+gcf671c0+chromium-156.0.8078.80_windows64_minimal.tar.bz2;
upstream archive SHA1 b690e035689ee20ba9d2500ac8953815bc7ed46a. Keep runtime
DLLs/resources/locales beside the client. Rename upstream bootstrapc.exe to
humanymous_cef_bootstrap.exe and copy the two Rust DLLs beside it. Copy upstream
cef-app.exe.manifest as humanymous_cef_bootstrap.exe.manifest. No C/C++ glue.
Both bootstrapc.exe/libcef.dll were observed Authenticode NotSigned: this private
pilot does not establish signed distribution, native CVE clearance or provenance
beyond upstream hash checking and recorded local SHA256.

RunConsoleMain follows the current four-argument ABI. It rejects weakening flags,
forwards the executable-owned sandbox pointer to CefExecuteProcess, and only in
the browser process loads the fixed sibling Meari desktop DLL. DLL_LOAD_DIR and
DEFAULT_DIRS exclude the working directory from desktop dependency search.
Desktop initializes CEF no_sandbox=0 with its Windows multi-threaded UI loop.
Meari owns winit. CEF creation is asynchronous through the scoped parent callback;
the consumer retains NativeParentLease and Meari retains the parent while pending
creation/cleanup. A late creation after cancellation immediately closes.

DoClose queues destruction of only the internal CEF child on CEF's UI thread.
Meari parent cleanup waits for on_before_close. A cancellable joined deadline
thread avoids outliving the desktop DLL. Timeouts/panics fail-stop nonzero rather
than returning a fabricated native cleanup ACK. OS resize checks only the owned
child; parent WS_CLIPCHILDREN prevents overlapping repaint feedback. No video
reservation, mock frame, external browser docking or authored script is used.

## Build and execution

Requires Windows x64 GUI session, Rust1.99.1, MSVC14.44/Windows SDK,
installed Visual Studio CMake/Ninja on PATH, and the pinned CEF runtime. Long
Windows object paths require a short private target/CEF path (the pilot used a
temporary SUBST alias). Do not disable sandbox/security to repair build problems.
After root integration:

`cargo build --locked -p humanymous-cef-bootstrap -p humanymous-cef-desktop`

Place outputs in the private bundle described above, then run
`humanymous_cef_bootstrap.exe --native-pilot-seconds=25` (5..180 allowed).
`--native-cancel-before-created` exercises actual cancellation during creation.
A fresh synthetic profile is created; an existing profile directory is rejected.
Reports are private native-report.json beside the executable. No personal profile
or cookies are supplied. The exact static input fixture is generated from Rust
Meari IR by its upstream markup builder; it is target test data, not the app shell.
JavaScript is disabled, other navigation/resources/popups/files/permissions denied.
Extensions/component updates/background networking are disabled. These flags do
not prove OS-enforced egress isolation or an ordinary browser baseline.

## Validation and remaining gates

Private source-linked harness: four library tests, warnings-denied Clippy, actual
DLL linking; one explicitly invoked native test rejects four weakening flags
before UI/profile creation. Actual Windows parent relationship, CEF child,
resize readback, focus-within-child, on_before_close and Meari cleanup observed.
Cancellation observed a genuine late creation followed by child-first cleanup.
Native OS roles/tokens identify renderer processes with restricted tokens and
Untrusted integrity RID0. This is limited sandbox evidence; complete sandbox
policy, utility-process isolation and renderer file denial remain unqualified.
CEF SystemInfo.getProcessInfo returned unsupported_or_failed; OS inspection is
reported separately and retains only bounded role/token metadata for rechecked
owned executable descendants, never full command lines.

Native UIA exposed the CEF document, named text input and Unicode value action.
Maximization propagated an additional resize readback. Keyboard text injection
had no confirmed value readback; UIA focus/cached bounds have residual issues.
Actual IME composition/candidate placement, screen-reader operation, cross-monitor
DPI, renderer crash/restart, all six native tuples and full AX qualification are
not_run/unqualified. Native permission callbacks are implemented but their real
prompt/file-denial matrix is not exercised by the script-free text fixture.
BrowserReady remains false. No server verdict or automation detection performance
is measured. Docker/backend integration must wait for actual qualified target
input/navigation and join the existing collector/session/server audit/origin lab.

Canon: .spec/sot/browser-ui.sot.md and
.spec/plan/20261107002014-browser-ui.plan.md. Native evidence is registered in
.spec/evidence/20261107012426-browser-ui-cef-native; raw sources/builds/outputs are
private in evidence:root/20261107004558. The initial ead2dc6 management-only slice
and its nine core tests remain historical evidence, not full browser completion.

Cleanup residual: automatic approval review blocked bulk recursive deletion of
previous synthetic-profile directories (reason only `blocked by policy`). No
alternative deletion was attempted. They remain private generated runtime cache;
do not publish/inspect their contents. Native child/window/process close succeeds
separately and must not be conflated with disk-profile deletion.

Repository-wide spec check remains blocked by pre-existing private documents outside
this lane (1509 errors, 1 warning at the recorded full run). Scoped browser public
and private evidence checks pass; this is not a whole-repository spec pass.

## Keyboard, IME, AX and sandbox follow-up (current)

Evidence: evidence:root/20261107021004 supersedes the earlier keyboard-unqualified
and renderer-file-denial-unverified observations only for this Windows x64 pilot.
The source-linked private harness passed seven native library tests, zero ignored,
Clippy -D warnings, fmt and DLL linking. Core nine tests remain applicable. Normal
and cancellation native runs exit0 with real on_before_close/Meari cleanup.

Actual native input path: Tab enters the CEF editable field and selects its value;
physical Latin keys mutate it. In --native-korean-ime mode, F8 opens the focused
owned CEF context in the already installed Korean native conversion mode. Physical
r -> k produces ㄱ -> 가; BackSpace returns ㄱ, k restores 가, Return commits.
No Unicode value action or script implements these keyboard/IME mutations. This
is an explicit pilot mode, not general automatic language switching or a keyboard
focus fix. Initial direct click/focus and full parent/child traversal still need
qualification. AX sometimes reports document focus and a stale value until a
fresh observation. The unproven pointer-based focus restoration was removed.

F9 in that opt-in mode asks the real IME for conversion and candidate opening
(ImmNotifyIME). Conversion returned true; candidate opening returned false. No
candidate window or candidate form was observed. Placement, candidate selection,
IME Escape cancellation, other IMEs, DPI/monitor cases remain unqualified. IMM
composition byte-length stayed0 even while the native Korean incremental edits
and BackSpace were visible; do not invent an IMM preedit receipt from this.
Native UIA role/name/value exposure and Unicode Value actions/readbacks passed for
both the CEF field and Meari address. A Meari tab click reported no cached bounds;
full focus/geometry, screen-reader speech and full AX qualification remain open.

A minimal Rust renderer handler now receives bounded owner metadata after native
renderer initialization. It directly attempts the generated canary read and a
create_new sibling probe file. The fixture renderer reported ERROR_ACCESS_DENIED5
for both and its PID matches the observed restricted/Untrusted renderer token.
A spare renderer had no browser callback/probe: do not count it as file-tested.
This does not prove all filesystem paths, utility-process policy, process/job
mitigations, network isolation, escape resistance or signed distribution. The
canary exists only as synthetic local data; no personal paths/content are read.

### Required explicit provisioned gate

The native rejection test is feature-gated, not ignored. Default workspace unit
runs contain zero ignored tests. A default unit pass does not discharge this gate:

HUMANYMOUS_CEF_PILOT_BUNDLE must be an absolute provisioned bundle path; then run
`cargo test --locked -p humanymous-cef-bootstrap --features native-pilot-tests tests::native_bootstrap_rejects_security_overrides_before_ui_or_profile_creation -- --exact`.
The case must execute once with1 passed/0 ignored and reject all4 flags before
UI/profile creation. It passed locally on the final bundle. Missing provisioning,
missing feature execution or a skipped job is NOT_RUN and blocks component
acceptance. Root owns scripts/ci/cef-pilot and its dedicated Windows component CI;
that gate proves build/negative startup only, not GUI/IME/AX/full sandbox support.
Root's exact-source local integration gate passed and was committed/pushed in ac560ff.

### Cleanup update

The user explicitly approved deleting generated private profiles. A second
PowerShell deletion command containing path validation was nevertheless rejected by automatic
approval review with only blocked by policy. It did not execute; no alternative
mechanism was attempted.32 generated profiles remain private. All CEF test
processes have closed; disk cleanup remains separate from successful child cleanup.

OSK follow-up on unchanged a924603 source: the built-in Korean on-screen keyboard
exposed a Hanja key. Its click had no confirmed key-delivery/candidate receipt.
Candidate qualification remains open. The owned CEF pilot exited0. OSK UI-close
had no confirmed effect; stopping only its introduced PID/start-time matched
process returned access denied. No elevation/security bypass was attempted; the
introduced OSK window remains for manual close.32 generated profiles now remain.

Shared integration is committed/pushed in ac560ff with browser source a924603.
Root evidence .spec/evidence/20261106235519-program-status/cef-root-integration-results.json
records DLL build, native7/0ignored, Clippy and explicit1case/4flags pass. Its native
source SHA256 map was compared with this evidence and matches. Root artifact
hashes differ from the source-linked GUI bundle and have separate scope. The new
remote exact-integration CI is pending; neither that component gate nor native
Linux stubs establish full browser/IME/AX/sandbox or six-tuple runtime support.

## Hanja keycode research correction

The user confirmed a physical Hanja key. Official Windows VK_HANJA=0x19(25),
VK_KANJI=0x19 and VK_HANGUL=0x15(21). VK_RCONTROL=0xA3 is distinct; a layout
shortcut must not be assumed equivalent to the Hanja virtual key. The documented
computer-use API accepts keysym strings, not raw VK/scan-code arguments. Standard
Hangul_Hanja (X0xFF34) and Kanji (X0xFF21) were both tried on the owned CEF input
with native Korean 가 present; both returned unsupported key before delivery.
This is a tool mapping limitation, not an unknown/nonexistent Hanja key. No native
raw input helper, direct key-message injection or inferred hardware scan code was
used. A supported VK0x19 delivery path or an observed physical Hanja event is
needed to qualify actual candidate display/placement. Candidate remains open.
The unchanged-source pilot exited0;33 private generated profiles now remain.
See hanja-keycode-research.json in evidence:root/20261107021004 for primary links
and the exact unsuccessful aliases. No application source changed.
