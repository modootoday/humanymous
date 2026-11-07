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
| Hook handoff | exact method/target/authority/header/body binding remains unchanged; main request only; no quick-cache bypass; consume shared authority result once and recheck liveness before effect |

Every extern callback catches unwinding and returns static failure. OOM abort,
panic=abort, SIGKILL and memory corruption cannot be caught; do not claim clean
recovery or audit completion for those cases. Native pool cleanup and request
callback serialization are host premises that require actual MPM pilots.
No third-party probing, security engine duplication or sandbox weakening.
