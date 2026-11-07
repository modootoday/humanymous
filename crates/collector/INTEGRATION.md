# Rust browser collector integration

`humanymous-collector` is a root workspace member with pure L1-L3 evaluation and
a wasm32-only browser I/O boundary. It reuses `humanymous-signals` constructors,
registry and `ClientReport`; no scoring implementation is duplicated.

The exported `detect_json` reads actual browser APIs and returns a partial report.
Browser getter/call errors reject. The report has no minted session and leaves
environment.probed=false and behavior/advanced/guard unmeasured defaults.
The two CDP flags are only consumed: their active producers remain unported.
This is not a qualified runtime collector, human proof or full Pass implementation.
Required runtime collector routes remain fail-closed rather than accepting this
partial report as a substitute for missing evidence.

All authored implementation/tests are Rust. wasm-bindgen0.2.130,
js-sys0.3.107 and wasm-bindgen-test0.3.80 are pinned. Production js-sys disables
default features including unsafe-eval; upstream generated binding/test glue is
an external artifact, not a hand-authored JavaScript loader. No eval/inline_js or
Function constructor is authored. Go detect files remain read-only migration oracle.

Pure tests cover all17frozen signal IDs, values/confidences/verdicts and registry
weights, benign/absence/zero/case boundaries, software-renderer Unicode lowering,
memory values and fresh-realm distinctions. Actual browser tests cover API reads,
primitive conversion/budgets, and a Rust-defined failing getter after real iframe
insertion, proving propagation and removal on that error path.

Reproduce:

- `cargo test -p humanymous-collector --locked`
- `cargo clippy -p humanymous-collector --target wasm32-unknown-unknown --all-targets --locked -- -D warnings`
- `docker compose -f deployments/lab/compose.collector.yaml --profile collector build`
- `docker compose -f deployments/lab/compose.collector.yaml --profile collector run --rm --no-deps collector-browser`

The Docker component lab uses Firefox ESR and geckodriver, UID65532, network none,
read-only root, cap-drop ALL and no-new-privileges. Loopback WebDriver/test-server
traffic stays inside that container. There is no sandbox-disabling flag. This
does not attest the full browser sandbox policy.
The image's Firefox version command reports user-namespace creation EPERM under
the container's restrictions; namespace sandbox availability is not established.
No security setting was weakened to suppress that diagnostic.
Firefox/WebDriver preferences and headless operation alter observations and are
not an ordinary-human baseline.
The standard wasm-bindgen runner Chrome/Edge branches append no-sandbox; they are
prohibited here. Firefox is only a component-test engine; product UI stays Meari+CEF.

Remaining: full loader/distribution, L4 events, environment/guard/advanced probes,
session-bound collector/RIT transport/recovery, actual Pass, ordinary-browser/AT
controls, and product Docker e2e. A native/WASM build or component test does not
complete these. Public summary and source manifest are in
evidence:root/20261106235519; raw logs are private evidence:root/20261106194410.
