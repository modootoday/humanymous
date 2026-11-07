---
id: sot:root/apache-adapter
title: Apache native adapter lifecycle and authority boundary
status: active
verify: manual
created: 2026-11-07T07:26:00Z
updated: 2026-11-07T07:26:00Z
rel:
  governs: [crates/apache-adapter/**, adapters/apache/**, .github/workflows/apache-validation.yml]
  depends_on: [sot:root/rust-migration, sot:root/adapter-service-contract]
---

# Apache native adapter

## Standalone hosted qualification entrypoint

The root/CI-approved narrow workflow ownership is
.github/workflows/apache-validation.yml, named CI / Apache component. It runs
only on codex/apache-validation pushes or manual dispatch, on standard Ubuntu
24.04 with read-only contents permission and pinned checkout/toolchain actions.
Root alone pushes the reviewed exact commit. No publication, deployment, OIDC,
private evidence upload or automatic artifact upload occurs. Bounded safe
coordinator receipts remain in the actual job logs.

Safe lifecycle units and pilot receipt/engine units, Linux pilot all-target
check and warnings-denied Clippy precede the sole coordinator. The coordinator
creates a fresh0700bundle at the actual canonical RUNNER_TEMP/apache-native
for a genuine hosted non-root actor, binds Git archive HEAD to GITHUB_SHA and
runs the existing Docker ABI/MPM fixture. Its separate owned-ledger cleanup
runs even after a failed started pilot step. Missing cases, failed process
exits, source/artifact drift and cleanup failure remain failures. An incomplete
or unrun component never establishes a qualified Apache tuple.

Version0.1 experimental target; no Apache tuple is currently qualified.
Authored implementation/tests are Rust1.99.1 edition2024. Native ABI unsafe is
approved only for a separate reviewed native crate by root on2026-11-07; the
safe lifecycle crate retains unsafe-forbid. ABI-SAFETY.md defines per-operation
preconditions. Safe lifecycle implementation does not establish module support.

## Required lifetime contract

Child initialization occurs after fork. A child owner binds actual process ID,
generation and supported MPM. Parent-created runtimes, credentials and threads
must not be reused after fork. Prefork admits one request; worker/event admit
a configured bounded number. Admission is atomic and is retained through input,
authority await and origin handoff. Graceful child shutdown closes new admission
and invalidates outstanding work. A replacement child owns a new generation.

Each request owns bounded copied input, exact method/target/authority/header
observations and a single monotonic budget. No request_rec/APR pool pointer may
cross an await, thread or pool cleanup. Pool cleanup cancels the request, and
origin handoff consumes an authenticated authority response exactly once.
The safe request owner is !Send/!Sync, enforcing host-thread replay affinity.
A separate idempotent cancellation signal can be retained by pool cleanup while
authority work is pending. Cancellation retains admission until request drop;
it cannot revive work or mint an authorization. Native cleanup registration is
authored but unexecuted and must be measured separately from these pure contracts.
Initial source rejects loaded mod_ssl as well as mod_cache: its plain HTTP/1
socket EOF probe does not establish TLS close-notify cancellation coverage.
MPM state and generation are queried again before handoff; stopping revokes the
safe child owner on the request thread, avoiding mutex use in a signal callback.
The graceful pilot must observe authority work in flight before requesting
reload, deny that old-generation origin effect, and then exercise a new child.
Subrequests and internal redirects do not inherit authorization; initial native
coverage rejects them until separately qualified. Cache quick handlers and
request rewrites must not bypass guard or change an authorized request binding.

Normalized Apache observations use provisioned Apache profile only:
humanymous.apache/0.1.0-experimental.1, apache-normalized, rawHeaderOrder=false,
peer/connectionId/capture/stream=null. Do not impersonate Node or claim original
TLS/H2/header order. Shared framework encoding and authenticated authority client
own validation and decisions. No scoring or transport/crypto clone.

## Adversarial requirements

P0: serving before security hooks, mutable target after decision and inherited
subrequest decisions are bypasses. Required: initial hook coverage plus final
binding check, no cache path, explicit subrequest rejection.
P0: forked runtime locks and retained APR pointers cause stale/process-unsafe
access. Required: child-only construction, owned copies and cleanup cancellation.
P1: truncated/overflow body must poison request, never authorize prefix bytes.
P1: request capacity must remain held until final effect, including cancellation.
P1: event MPM does not make blocking authority IPC asynchronous. Initial blocking
worker behavior must have explicit deadlines and measured capacity; no event-loop
scalability claim. Authenticated ALLOW alone grants no management/recovery access.

## Qualification

Pure units cover lifecycle races, process mismatch, capacity restoration,
body truncation/overflow and one-shot transitions. Cargo runs only in root slot.
Actual native ABI, startup/reload, each MPM, real binary body replay, concurrent
requests, authority outage/timeout/expiry, cancellation, spoofed headers and
origin witness denial require source-bound free hosted GitHub Docker execution.
Record exact httpd/APR/libc/architecture/module/toolchain/image identity and all
failed/unsupported/ignored/not_run cases. Compose belongs to product-lab owner.
Native ABI approval, workspace registration, shared Apache profile and hosted
runner integration are required dependencies, not implicit waivers.

References: https://httpd.apache.org/docs/2.4/developer/request.html and
https://httpd.apache.org/docs/2.4/developer/modguide.html.

## Native implementation checkpoint

Bounded body replay supports READBYTES, GETLINE and SPECULATIVE semantics without
copying the complete body or releasing admission. It remains an input mechanism,
never a grant. Actual filter installation must follow consumed authority ALLOW.
Nine pure tests and safe all-target Clippy passed in root-assigned A. A later
root-assigned D repeated9pure passes and verified the registered native package's
DEFAULT all-target check/Clippy without headers or bindgen. Exact source/graph
receipts are evidence:root/20261107082159 and evidence:root/20261107085316.
Those historical default results did not select httpd or execute the45scenarios.
Native source inputs are pinned in adapters/apache/upstream-lock.json using
official release checksum pages and observed peeled Git tags. Configured header
closure and builder/clang checks now ran inside the actual SDK build, producing
an independently hash-bound ABI receipt. This does not qualify every host tuple.

## Actual hosted outcomes and MPM capability correction

Hosted source8042c6bc completed actual production/probe module builds/Clippy and
all45pairs with23passed/22failed/0not_run. Source d65da705 added only bounded
case projection and observed the same totals: event14/15, worker8/15,
prefork1/15. Safe9and coordinator11units passed with zero ignored/filtered;
pilot cleanup remained false while outer container/image and ledger cleanup
passed. Evidence:root/20261107113711 retains the exact source, manifest, image,
five artifact hashes, every case result and fixed failure labels. No runtime
tuple or full product success is qualified.

The pinned worker and prefork query implementations do not implement
AP_MPMQ_IS_ASYNC; their default returns APR_ENOTIMPL, which ap_mpm_query
propagates. Requiring that query for every child therefore rejects those known
models. Select only the exact host-reported event/worker/prefork identity and
independently validate threading/count/generation. Query async capability only
for event, require its exact affirmative value, and reject unknown identities,
missing event capability, inconsistent threading or zero capacity. No query
failure is converted into a guessed supported MPM. Three new pure capability
boundary tests make the next safe denominator12; they are not yet executed.

Primary pinned sources:
[worker query](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/server/mpm/worker/worker.c#L309-L360),
[prefork query](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/server/mpm/prefork/prefork.c#L269-L319),
[query wrapper](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/server/mpm_common.c#L393-L402).

The event failure is separately bounded to subrequest-rejection. The test-only
probe passed NULL next_filter to lookup, which runs normal internal processing
and records fixups failure in the subrequest status. ap_run_sub_req does not
check that status before invoking handlers/finalizing response protocol. The
correct probe observes actual503rejection, destroys that subrequest once and
sets its proof marker only after the observation; it never runs an already
rejected lookup. The unchanged test still requires non-200, that proof marker
and no origin effect. The earlier combined assertion does not establish a
production authorization bypass or a particular unrecorded HTTP status.
[Pinned request lifecycle](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/server/request.c#L2112-L2138),
[subrequest execution](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/server/request.c#L2348-L2369).

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