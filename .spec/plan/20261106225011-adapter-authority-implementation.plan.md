---
id: plan:root/20261106225011
title: Adapter authority implementation
status: active
created: 2026-11-06T22:50:11Z
updated: 2026-11-07T03:39:00Z
rel:
  governs: [crates/authority/**]
  depends_on: [sot:root/adapter-authority]
---

# Adapter authority implementation

## Goal

Implement the complete bounded local Node authority contract and record remaining
platform/product qualification gates without claiming migration completion.

## Steps

- [x] Claim gate-edge / rust-authority and root Cargo ownership.
- [x] Read canon, shared APIs, design personas and reflect critical review.
- [x] Publish early binary/transport/readiness contract in crate integration note.
- [x] Implement credential epoch, route readiness, replay, shared scoring/audit.
- [x] Implement bounded HTTP and native Unix/Windows listener security.
- [x] Run unit and actual process security/restart pilots; advisory/MSRV checks.
- [x] Record Docker/native gaps and checks; prepare attributed own commit.

## Record

No Go or public docs edits. No scoring policy changes. Docker was absent from the
Windows PATH but available inside WSL REA-Ubuntu (Docker 29.1.3). Linux x64 tests
ran in actual network-none containers, not WSL host authority. New interprocess
2.4.7 declares MSRV 1.75; actual Windows and Docker compilation/tests used 1.88.

Windows x64: 15 passed (3 unit, 7 decision, 5 actual-process). Linux x64 Docker:
14 passed (3 unit, 7 decision, 4 process), with 2 explicit privileged/helper
ignores. The privileged parent was then explicitly run in a separate network-none
container with only SETUID/SETGID: 1 passed; its nested helper actually executed
as UID 65534 and observed PermissionDenied. The helper is not a second product
case. No ignored test is included in passed counts. Both containers exited 0.

The first Docker run found Unix reset after capacity rejection with unread bytes.
The framing owner now drains only the rejected bounded frame through fixed
scratch storage before 429, under the rejection deadline. Final tests passed.
Clock rollback poisons the epoch; canceled queued work checks cancellation
before reservation and audit. Broken clients during native credential lookup
close only their own connection, rather than crashing the listener.

Cargo Clippy warnings-denied, workspace formatting and workspace tests passed on
the shared Windows checkout. The last authority/MSRV and Linux runs cover the
final runtime sources. Cargo audit found zero vulnerabilities/warnings using DB
commit 309392b18f6b17022f82c701f300433126901068, updated 2026-11-06T16:34:32+01:00.
That is a Rust dependency result, not a container OS vulnerability scan. The
MSRV compiler image is a test-only pinned base; it is not a published runtime.
Private raw evidence is retained in .spec.local/root/evidence/
20261106230114-adapter-authority-local-security-pilots (not a public dependency).

## Handover

Mission: provide the actual local Rust authority for the existing Node protocol.
Implemented crate/binary, strict framing, scope/peer/epoch/replay/route/capability
checks, shared engine/network/parser and durable shared Log now exist. Read
AGENTS.md, .agents/lessons/HARD-WON.md, authority SoT and
crates/authority/INTEGRATION.md before integration. No scoring change occurred.

Verify: cargo +1.88.0 test -p humanymous-authority --locked;
cargo clippy -p humanymous-authority --all-targets --locked -- -D warnings;
cargo fmt --all -- --check; spec check. For isolated Linux: build the crate's
Dockerfile.pilot from repository root, then docker run --rm --network none
--cap-drop ALL --security-opt no-new-privileges humanymous-authority-pilot:local.
The explicit other-user command is cargo test --locked --offline -p
humanymous-authority --test native_process unix_other_user_is_denied_by_actual_socket_acl
-- --ignored inside a separate root test container with only SETUID/SETGID.

Compose owner has its own authority image/Node pilot under deployments/lab and
build/rust; no ownership edits were made there. It can consume the documented
binary/startup JSON/private readiness interface. nginx's existing pilot protocol
is incompatible; future capture support requires a versioned contract. Windows
supervisor supplies actual Node PID/SID and private stdin/stdout handles; Unix
uses configured same-UID peer and a 0700 directory/0600 socket. Abrupt-exit stale
socket cleanup is a trusted supervisor operation after verifying process exit.

Remaining gates: native macOS/ARM tuples; Windows real different-user and remote
host denials/token/handle lifecycle; independent witness/anchored-head deployment;
real Node/Bun tuple qualification; browser collector/session/RIT data-plane and
challenge recovery; full isolated Gate/browser/Pass/wargame product lab. Required
missing TLS/H2/collector/RIT always returns 422. The production application-http
profile never synthesizes collector evidence and currently yields frozen HR-10
CHALLENGE for the positive HTTP fixture. Only the in-process synthetic test
provider yields fixture ALLOW and proves durable ordering. Administration is
unavailable, including erasure/kill/permanent-ban operations. No publish, main
merge, tag, paid infrastructure or production change occurred.

Public-doc impact: describe the exact experimental profile, supervision/IPC
permissions, dependency/clock/audit failures, unimplemented browser/admin
surfaces, and evidence distinctions; docs/** was deliberately left untouched.

Final specification gate: canonical spec check --no-local passes. Full spec
check was run and reports only unregistered raw files owned by other active
sessions (root migration and the earlier Node evidence bundle). This authority's
own raw files are attached and its canonical documents have no outstanding error.
Do not repair another owner's evidence bundle or describe the global check as
passing until those owners register their attachments.

## Subsequent guard-lifetime repair

Commit d179562 replaces close-only custody/WAL lock ownership with a shared
explicit-release guard. Historical receipts above qualify only their original
source. The reproduced duplicate-description failure, same-inode/successor
regressions, Linux/Windows checks and executed other-UID helper proof are in
evidence:root/20261106235519/custody-lifetime-results.json and
plan:root/20261107031001. Exact patched-commit native CI remains pending.
