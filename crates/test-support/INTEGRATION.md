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
