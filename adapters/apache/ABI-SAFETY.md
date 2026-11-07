# Apache native ABI review boundary

Root approved this narrow boundary in this chat on 2026-11-07. Safe lifecycle
crate keeps unsafe-forbid. A separate native crate uses unsafe-deny and allows
unsafe only in its reviewed host module and generated upstream bindings.
Manifest/dependency registration remains queued to root's next graph window.

The pinned source tuple is httpd2.4.70/APR1.7.6/APR-util1.6.6 on Linux x86_64 GNU.
Release archive hashes, peeled upstream tag commits and directly downloaded
header SHA256 are in upstream-lock.json. bindgen0.73.2 is the proposed locked
generator. Hosted builder image, clang executable hash and complete configured
transitive header closure have not been observed; they must be retained before
native build qualification. Source pins are not proof of the installed ABI.
All upstream sources use Apache-2.0; preserve upstream LICENSE/NOTICE in artifacts.
Existing Cargo.lock records bindgen0.73.2 package checksum
787ef8ef523575546b106a58213d6e6b06198a05c2f757258c68a74273670cfa.
Build source validates the direct pinned headers, observes the loaded libclang
path through proc maps, and hashes all reported transitive included headers.
No build receipt exists yet; these checks are authored and unexecuted.

| Operation | Required host precondition and ownership |
|---|---|
| Export module/commands | httpd loads the exact generated ABI; master exclusively initializes module index before callbacks; no concurrent Rust reference to mutable export |
| Read request/server/config pointers | host supplies aligned initialized live matching-ABI objects exclusively valid for this synchronous callback; null/length checks do not prove allocation validity |
| Read host strings/tables | host owns initialized NUL-terminated strings and declared array allocation; copy within explicit byte/count budget; never send pointers across async/thread boundaries |
| Pool cleanup bridge | one Box is registered once in its matching live pool; callback consumes exactly that allocation once; cancellation signal carries no APR pointer |
| Input replay brigade | callback owns live brigade/filter on request thread; APR-created buckets inserted only using matching host ring layout; copied bytes outlive all bucket reads |
| Child initialization | construct after fork only; expected PID/generation is checked before locking; no inherited Tokio runtime/thread or credential from parent |
| MPM identity and queries | read bounded host-owned ap_show_mpm name; accept only pinned names with consistent threading/nonzero capacity, and require event async query success; worker/prefork never fall back from a failed query to a guessed model |
| Hook handoff | exact method/target/authority/header/body binding remains unchanged; main request only; no quick-cache bypass; consume shared authority result once and recheck liveness before effect |
| Test-only nested request probes | opt-in pilot-probes build only, after consumed parent ALLOW; upstream allocates the lookup-only subrequest in its matching pool; observe real fixups rejection503 before setting the proof marker, then destroy once; never run a rejected lookup or retain its pointers; internal redirect callback must reject inherited decisions |

Every extern callback catches unwinding and returns static failure. OOM abort,
panic=abort, SIGKILL and memory corruption cannot be caught; do not claim clean
recovery or audit completion for those cases. Native pool cleanup and request
callback serialization are host premises that require actual MPM pilots.
No third-party probing, security engine duplication or sandbox weakening.

## Owned fixture control and isolated CLI observation

Hosted source2d66c0df run42369797560 observed33passed/12failed/0not_run:
event15/15, worker15/15, prefork3/15. Safe12/native11 tests and SDK builds/
Clippy passed. The first prefork CLI stop failed after a successful nested
assertion; later connection errors are a cascade, not twelve proven causes.
Historical CLI streams were discarded by Stdio::null and cannot be recovered.
Evidence:root/20261107120442 retains those failed outcomes.

Root approved existing rustix pidfd process control for owned pilot instances.
Capture the handle immediately after spawn, before any wait/reap; exclusive
std::Child wait ownership reserves even an exited child's PID through capture.
Capture failure cleans the sole unreaped Child and fails, without normal
numeric-PID fallback. TERM stops; USR1 requests graceful restart, with the same
actual master-exit0/new-generation/inflight-drain/deadline assertions. A forced
KILL is failed cleanup. A released owner never signals again. No dependency,
unsafe boundary, detection score, resource bound or 45-case inventory changes.
[Pinned Apache graceful signal](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/include/mpm_common.h#L68-L75).

Before those45cases, a separate fresh prefork master/config/private directory
repeats the nested request and observes the legacy stop CLI, only after its
PID file matches the exclusively unreaped owned master. Capture bounded private
stdout/stderr, actual helper exit/signal, master before/after/reap and cleanup
facts. Never reuse that stopped master for required cases or count a46thcase.
This is an isolated CLI observation, not general CLI qualification. Only closed
typed facts and allowlisted Apache codes may enter public output; byte carriers
stay in the private coordinator stderr and decoded mode0600 files. A malformed
or incomplete diagnostic fails overall completion but preserves independently
verified45-case counts. Any actual CLI failure remains visible as such. New
source is unqualified until scoped pure checks and an exact-source real hosted
retry complete; earlier33/12 evidence remains failed.