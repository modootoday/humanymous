# Bounded process support

`process::{ManagedChild,capture,capture_with_stderr,Captured}` owns/reaps the
direct native process, observes actual exit status and bounds readiness/stdout/
stderr/deadlines. It is test/pilot support, never a product service supervisor.
`capture(&mut Command,input:&[u8],timeout:Duration,limit:usize)` suppresses stderr;
`capture_with_stderr` uses the same implementation and returns both streams.
`Captured{status,stdout,stderr}` has content-redacted Debug (only status/lengths).
Callers retain raw stdout/stderr only in private evidence, not public diagnostics.

Private startup stdin is limited to4096bytes before launch; each output stream
has its own byte cap. Overflow/unknown exit/EOF after the total deadline cannot
become truncated success. A descendant-owning caller must additionally own its
job/process group. This helper kills/reaps only the direct child and does not
claim arbitrary descendant containment. No foreign script strings/dependencies.

Eightlocal Windows tests passed, including actual child nonzero exit, bounded
hang kill/reap, stdout/stderr overflow, concurrent stderr and content-redacted
Debug. Native Linux/macOS execution remains required. Root/lab can consume the
existing workspace dependency without changing this crate's dependency closure.

New API: ManagedChild::spawn_duplex(&mut Command) returns guard,ChildStdin,
ChildStdout with stderr=null; try_wait observes actual status. The typed witness
client owns frame size/deadline/pipe close and the shared guard owns kill/reap.
Nineactual local helper tests pass including duplex echo/EOF. This does not give
interactive I/O an implicit deadline; the protocol owner must enforce one.

WASM boundary: process is cfg(not(target_arch="wasm32")); the JSON assertion
library remains available to WASM. OS child APIs cannot be invoked on that target.
This fixes target-specific drop_non_drop without allowing lints or pretending an
OS actor exists. wasm32-unknown-unknown all-target Clippy and host9tests passed.

`json::decode_strict_json(bytes,max_bytes) -> Result<Value,StrictJsonError>` is the
shared report syntax boundary, extracted with attacker-owner consent from671c439.
TooLarge is decided before parse; Invalid covers duplicate decoded keys at every
depth, syntax/nesting/number/trailing failures. Errors never retain input fragments.
Serde's recursion limit remains enabled; escaped equivalent keys collide and u64
precision is retained. Consumers own schema/unknown-field/replay verification and
their byte budgets. Attacker removed its original visitor in052c1da; lab uses the
same API rather than a second visitor. New shared two pure tests are not yet run;
root lock/slot integration is required. The old9process tests do not qualify this API.

Subsequent canonicalf44b32b sharedJSON pure2 passed (0failed/ignored,9filtered) and
all-target warnings-denied Clippy passed. No process test was executed in this
selection. Post-run owned source bytes match canonical Git blobs; exact counts,
hashes and private-log registration are in the cross-os runner evidence bundle.

## Owned-tree model and read-only prerequisite inventory (unqualified)

`tree_lifecycle::Lifecycle<Owner>` is a PURE symbolic model, not a native resource
adapter. It retains the owner on incomplete finish, borrows observations, and
requires six separate terminal obligations. SDK execution success is separate
from cleanup. No custom Drop, spawn, kill or recovery supervisor exists; actual
consumers must not use it to bypass the blocked native backend contract. Four
new pure tests passed in the allocated native Windows selector.

`linux_tree_probe::inspect()` reads fixed PID1/mountinfo/socket prerequisites
without subprocesses, D-Bus calls, policy prompts or filesystem mutations. Reads
have byte bounds, not a cancellation/wall-clock guarantee. Manager version and
broker authorization are explicitly NotQueried, and NativeBackendUnqualified is
always returned. Two pure inventory tests passed on Windows; Linux cfg and inspect were not run. No supported
Linux backend, actual prerequisite result or genuine-host claim is established.

Both new selectors passed with all-target check and warnings-denied Clippy; ten
source/manifest/lock/toolchain files had zero drift. Exact evidence is registered
in the cross-OS runner owned-tree-pure-summary.json.

## Shared unverified parent context

parent_context owns decode/encode, typed workload/cleanup wire, named source
lineages, fixed environment and one local mapping of original180s+60s claims.
All decoded and mapped types are UNVERIFIED: no parent/artifact/token/Job/Ready
or original-clock authority is created. Runner retains native/path/hash admission;
.NET consumes this same wire rather than another codec. Shared4 plus runner7
passed on immutableb482+owned overlays with BOTH check/strict Clippy; actual OS
execution is unqualified. Exact source hashes are registered in cross-OS runner
parent-context-joint-summary.json. Do not mutate these shared bytes during the
.NET consumer's separately allocated source-bound compile.
