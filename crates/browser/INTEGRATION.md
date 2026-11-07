# Browser integration request

Owner: misc / rust-browser-ui-next. Status: management UI/guard source implemented
and workspace-integrated; 9 locked tests plus Clippy/format pass. Actual Windows
x64 Meari UI observed. CEF target browsing/adapter remains NOT READY.

Root delta: add `crates/browser` to workspace.members. Keep publish=false.
The crate uses workspace package metadata/lints. Desktop deps are optional behind
`native-ui`: `meari` and `meari-native` from
https://github.com/modootoday/meari.git revision
1482b746ebc0d867f25d17d01c48c12df888ca6c; meari-native features=["window"],
default-features=false. `url = "=2.5.7"` is the policy parser dependency.
No shared Cargo files are edited by this lane. CEF dependency is not yet added:
artifact/features/MSRV/bootstrap qualification is pending, no implicit download.
Dev dependencies: accesskit=0.25.1, blitz-traits=0.3.0-beta.4,
keyboard-types=0.8.3 (all exact pins; match Meari closure).
Dev dependency `meari-dom` uses the same Meari git revision and
default-features=false; its library name is `blitz_dom`.

Progress: detached harness directly referencing the actual crate sources and
the exact checked-out Meari source passed 8 tests. Native window creation and
scenario-panel switching were observed on Windows x64; host fonts were used
because the pinned Noto cache is absent. UIA exposes named address/tabs but
static text propagation/bounds need qualification. Initial CEF check failed for
missing PATH CMake, retry failed for long native object paths. Using the installed
VS CMake/Ninja and a temporary private-path SUBST mapping, real upstream
`cargo check -p cef --no-default-features --features sandbox` passed with1.99.1.
That proves neither native executable linking nor actual sandbox/attach readiness.
CEF native distribution resolved to
`cef_binary_156.0.15+gcf671c0+chromium-156.0.8078.80_windows64_minimal.tar.bz2`,
upstream archive SHA1 b690e035689ee20ba9d2500ac8953815bc7ed46a.
Full artifact signing/provenance and SHA256 inventory remain required.

Final native pilot: integrated workspace executable, synthetic input only. Named
UIA address, headings/status and tabs; native address focus/typing, Tab+Right panel
navigation, scenario switch, and asynchronous Alt+F4 close were observed. Explicit
aria labels repaired static-text names. Indexed UIA click bounds remain a tool/AT
qualification residual. Injected IME preedit/commit unit case passes; actual OS
IME candidate placement, screen reader and native DPI remain unrun.

Root continuation: use `.spec/plan/20261107002014-browser-ui.plan.md` and
`.spec/sot/browser-ui.sot.md` for exact CEF lifecycle/input/AX/bootstrap contract.
Keep B2 partial/notready. The native dependency closure check is not CEF runtime
or sandbox proof. Docker/backend integration request: after real attach, drive
owned fixture through collector/session and join server audit/origin receipts;
today no target requests or server integration can be honestly tested here.

Binary: humanymous-browser, requires native-ui. Build/run after root integration:
`cargo run -p humanymous-browser --features native-ui --bin humanymous-browser`.
Optional `--pilot-seconds 10` closes the synthetic management window automatically.
`cargo test -p humanymous-browser --features native-ui` exercises real Meari IR.

Prerequisites: Rust1.99.1, Windows MSVC C++ linker/SDK (upstream dependencies),
working native GUI session/fonts. Linux requires display and fontconfig/native
renderer libraries; macOS needs Apple SDK. Exact resolved native closure remains
to be measured. No C/C++ application glue is authored. Upstream crates may contain
native code; their notices/artifact provenance need the final dependency audit.

CEF path: Windows supported upstream bootstrap plus client DLL sandbox; macOS
signed helper initialization before framework load; Linux namespace/seccomp proof.
Never substitute no-sandbox. Consumer adapter lifecycle/input/AX contract will
remain separate from actual CEF qualification. No video reservation or external
window docking is permitted. Root must keep B2 partial/not-ready until native
attach, sandbox, IME/AX and owned backend integration pass.
