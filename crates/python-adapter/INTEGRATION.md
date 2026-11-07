# Python integration frontier

Native lib humanymous_python exports Rust Wsgi/Asgi classes with genuine Python
call protocol. No eval/from_code/script wrappers. WSGI detaches GIL during IPC;
ASGI schedules native callbacks on the owning event loop under copied context,
and Rust Drop schedules cancellation of the Python task. Runtime NOT_RUN.

Root workspace/lock registration required before locked build. PyO3=0.30.1 and
pyo3-async-runtimes=0.30.0 tokio-runtime. Build wheel with pinned upstream
maturin1.16.0 against adapters/python/pyproject.toml, production extension-module
feature. CPython3.12..3.14 regularGIL candidate only. No ABI3/free-threaded claim.
Candidate distribution name remains unverified in PyPI.

Hosted must import installed wheel and attach actual upstream FastAPI/Starlette/
Django/Flask objects using Rust object API calls/native callbacks, never Python
script strings. Use real server and authority with audit/effect counters.
Synthetic ALLOW stays distinct from production browser evidence.

Compiler, Clippy, pure units, contextvars/cancellation/GIL, fidelity, server read
deadline, actual frameworks, clean install, Unix/Windows IPC/ABI and lock audit
are all NOT_RUN. No previous Node success is attributed to this source.

Hosted smoke contract expects2groups4cases only for actual framework objects
and dependency-down, with explicit ignored opt-in. No server/ALLOW/audit claim.
Candidate direct pins are pilot-requirements.txt; retain hosted closure/audit.
Rust wheel driver build/inspect prints hash/member/tag receipt with generation
provenance=false and whole/runtime qualification=false. Clean install/import and
input-source lineage remain mandatory. Initial Rustfmt syntax typo was corrected;
compiler validation has not run here.
