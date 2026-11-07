# Python integration frontier

Native lib humanymous_python exports Rust Wsgi/Asgi classes with genuine Python
call protocol. No eval/from_code/script wrappers. WSGI detaches GIL during IPC;
ASGI schedules native callbacks on the owning event loop under copied context,
and Rust Drop schedules cancellation of the Python task. Runtime NOT_RUN.

Root workspace/lock registration is present. PyO3=0.30.1 and
pyo3-async-runtimes=0.30.0 tokio-runtime. Build wheel with pinned upstream
maturin1.16.0 against adapters/python/pyproject.toml, production extension-module
feature. CPython3.12..3.14 regularGIL candidate only. No ABI3/free-threaded claim.
Candidate distribution name remains unverified in PyPI.

Hosted must import installed wheel and attach actual upstream FastAPI/Starlette/
Django/Flask objects using Rust object API calls/native callbacks, never Python
script strings. Use real server and authority with audit/effect counters.
Synthetic ALLOW stays distinct from production browser evidence.

Native all-targets host-pilot check002 and final Clippy002 passed against the
source recorded in evidence:root/20261107075243 summary002. The initial check001
and Clippy001 failures are retained. No tests or native imports executed.
Package-tool all-targets check/Clippy also passed in summary003 after the approved
three dev path dependencies were integrated; no tests/imports/install executed.
Contextvars/cancellation/GIL, fidelity, server read
deadline, actual frameworks, clean install, Unix/Windows IPC/ABI and closure audit
remain NOT_RUN. No previous Node success is attributed to this source.

Hosted component contract now expects3groups7cases for actual framework objects
and dependency-down plus trusted-server lifespan, with explicit ignored opt-in.
No actual HTTP server/ALLOW/audit claim.
Candidate direct pins are pilot-requirements.txt; retain hosted closure/audit.
Rust wheel driver build/inspect prints hash/member/tag receipt with generation
provenance=false and whole/runtime qualification=false. Clean install/import and
input-source lineage remain mandatory. Initial Rustfmt syntax typo was corrected;
package-tool compiler validation is recorded in summary003.
Root-approved host isolation: PyO3/async deps optional behind native;
extension-module/host-pilot/package-tool all enable native. Default0tests do not
qualify anything; mandatory shared6pure plus native compilation and actual host
groups/install gates remain. Use --features host-pilot for the three test groups;
use --features package-tool --bin humanymous-python-package for Rust packaging.
Later source adds trusted-server transparent lifespan forwarding and DjangoASGI
target coverage; no lifespan runtime result is claimed. Wheel install-test uses
upstream pip in a fresh target and native API import, with explicit hosted opt-in.
It has not run; archive/import does not establish generation provenance or full
framework/protection/ABI qualification.
New source requires WSGI server disconnect callback and preserves one ASGI
disconnect receive across the authority decision/handoff. These cancellation
paths are NOT_RUN. host-pilot pilot_wsgi exposes an upstream --call factory for
real Flask/Django servers with native binary views/cookies/effect receipts. Private
input uses a configuration JSON STRING, not a permissive parsed-map re-encoding.
The approved graph window added existing native-security/audit/signals dev path
dependencies; root resolved the lock with zero new external packages. Separate
waitress-requirement.txt pins the official3.0.2 wheel SHA256 for hosted-only
installation. No package has been installed or executed. Its candidate
profile must enable channel_request_lookahead and bounded server input deadlines.

New Unix-only tests/wsgi_server.rs contains three ignored actual HTTP groups,
ten planned cases across Flask/Django: production authority non-ALLOW with exact
audit count/no effect, real-engine synthetic ALLOW with binary/two-cookie/effect
hash/audit checks, and dependency-down/unknown/reserved rejection. It reuses the
shared test-only Evidence/audit fixture. This Unix source was excluded by local
Windows/package-tool cfg and remains NOT_COMPILED/NOT_RUN. Timeout, body-abort,
pre-handoff disconnect and live-authority oversized/upgrade cases still need
implementation; server-pilot-contract keeps complete harness=false.
