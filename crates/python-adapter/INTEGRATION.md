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
waitress-requirement.txt pins the official3.0.2 wheel SHA256 for allocated pilot
installation. No package has been installed or executed. Its candidate
profile must enable channel_request_lookahead and bounded server input deadlines.

Unix-only tests/wsgi_server.rs now contains six mandatory ignored actual HTTP
groups/28 planned cases across Flask/Django, plus two pure HTTP framing groups.
It covers production non-ALLOW/audit/no effect; synthetic ALLOW binary/cookies/
effect hash/audit; live-authority unknown/reserved/oversize/upgrade; dependency
down; partial-body abort/input deadline; accepted IPC response timeout; and gated
ALLOW disconnect with a matched connected control. Host lifecycle and authority
orchestration live in separate support modules; Evidence/audit reuse is shared.
Summary007 records the first Unix check/Clippy success for this target on the
immutable8eb66811 source snapshot. All eight intended primary source inputs
matched;2202 regular-file hashes were identical before/after. CPython3.12.3 dev
tooling was available. This type-checks the actual Unix branches but does not
build/link a test executable or run any test, module import, server or installer.
Eight Rust groups must still execute without ignores/filters; type-checking is
not executable/runtime qualification.
Summary008 subsequently records a UID1000 private venv (Waitress3.0.2 with wheel
hash enforced, Flask3.1.3/Django6.1.3), native fixture extension, authority and test
executable builds. No global packages changed. Initial orchestration quoting/
line-ending failures were retained; LF private orchestration resolved them.
Source regular-file inventory stayed unchanged after builds, and installed
dependency versions/metadata/file hashes were retained. Executables were copied
beside the private module so shared target rebuilds cannot replace those bytes.
Actual execution was denied before launch at87.51% system commit (admission must
be below85%); no test/server/import result follows. Prepared artifacts can be
reused after explicit allocation and hash/resource verification, without rebuild.
The incomplete-body timeout case measures idle closure. It does not establish
a total network upload deadline against a continuously progressing slow body.

User subsequently authorized actual local Windows/WSL execution for comparative
measurements as well as hosted execution. Root allocation remains required;
the active benchmark quiet window permits no new build/runtime commands. The
historical HUMANYMOUS_HOSTED_NATIVE_PILOT opt-in name is unchanged and does not
attest where a run occurred. Product e2e remains isolated Docker-only.

Summary009 retains the first actual8eb66811 run:6passed/2failed,10rows/22cases,
exit101. The fake timeout socket lacked required0600 permissions; the empty
audit fixture incorrectly reopened intentionally empty history. Primary-only
fix7842b6a8 preserved accepted request-byte proof and compares exact fresh owned
empty WAL bytes/device/inode/owner/mode instead of relaxing production reopening.
Summary010 records a new immutable7842b6a8 snapshot, coherent rebuilt module/
authority/test executable, check/Clippy, and successful actual UID1000 WSL run:
8passed,0failed/ignored/filtered,6actualgroups+2pure,14rows/28cases,12.69s.
All2254 source-file hashes and reused dependency-file hashes stayed unchanged.
Owned cleanup found no task processes; allocation B returned. These later results
supersede the historical NOT_RUN statements for this WSGI component only; ASGI,
wheel clean installation and hosted qualification remain separate pending gates.

The host-pilot-only pilot_asgi factory is authored in Rust, not compiled or run.
It reuses bounded private factory input, minimized effect writer, owning-loop
async scheduler and the existing production Asgi guard. Starlette/FastAPI mount
a native binary echo ASGI origin via public APIs; this scope covers ASGI routing/
middleware, not FastAPI parameter validation or dependency injection. Django
uses its standard ASGI application and the shared native Django view. The factory
refactor also changes WSGI fixture source; summary010 remains bound to historical
7842b6a8 and does not qualify the new bytes. Actual ASGI server/version, HTTP/
disconnect/deadline/lifespan cases and shared lifecycle verification remain open.

The dedicated workflow's runner.temp target expression now appears only in step
env maps, after actionlint rejected the original workflow-level context. Direct
actionlint passes after correction. The controller still refuses execution as
NOT_ACCEPTED until the shared atomic owned-lifetime API is reviewed. Neither
actionlint nor source formatting is compilation or runtime evidence.
