---
id: plan:root/20261107080000
title: CLI provisioning slice
status: active
created: 2026-11-07T00:00:00Z
updated: 2026-11-07T00:00:00Z
rel:
  implements: [sot:root/cli-installation]
---

Implement cohesive provisioning module and command parser with SHA-256 artifact
boundary. Verify in an isolated workspace until root integrates crates/cli.
Record Windows fixture pilot and negative tests privately. Root owns membership,
lockfile and full workspace acceptance. No Go, docs, service or scoring edits.

Implemented doctor/install/rollback with read-only plan, caller SHA-256, explicit
local-management acknowledgement, native private parent checks, path/reparse
rejection, create-new collision prevention and failure cleanup. Six Windows unit
tests and a real CLI temp-fixture pilot passed in an isolated workspace. Evidence:
private CLI evidence bundle 20261107080001. Public integration request is crates/cli/INTEGRATION.md.
Root integrated membership; actual workspace package tests (6) and Clippy with
warnings denied and --locked passed. Full workspace checks, Linux/macOS pilots, real server local management/IPC,
release artifact provenance and privileged/service registration remain open.

## Service follow-through (2026-11-07 KST)

Root reassigned CLI to the genuinely free sot-plan lane, disjoint from the Windows
runner's release lane. CLI owns release-descriptor verification, immutable service
generations, native manager adapters and explicit Rust process/service fixtures.
Shared native-security remains another writer; its accepted grant/revoke/resolve
API and strict custody boundary are recorded in the CLI SoT. Root owns Cargo/lock
and toolchain. Native-cli/framework scaffolding is out of scope.

Implemented: strict exact-byte Ed25519 manifest, host/digest/sequence/name/argv/
loopback readiness binding; service-plan/install/upgrade/doctor/rollback/remove;
existing installer reuse; explicit activation, private state/journal/create-new
lock, known-predecessor rollback with preserved sequence floor, manager collision
and configuration-change refusal, failed registration/readiness cleanup and old
service restoration. Contexts are release, installation, service/storage,
service/backend and command composition. No authored shell or executable-script
strings are used. OS service definition strings are declarative configuration.

Final Windows actual workspace:15library+1composition tests and2actual process
pilots passed;1positive native service pilot is ignored by default and remains
unexecuted locally. The service manager state-machine tests are explicitly mocks.
Windows real process proves signed plan/dry-run names/content/mtime/DACL unchanged,
native fixture readiness and actual SCM permission denial with staging cleanup.
Attempt11 final current-source tests, warnings-denied Clippy, format check and
default release binary build passed. The default build emitted only humanymous,
excluding the opt-in fixture. Working-source manifest before/after those checks
was identical: d68b8c9a25d714009291b52b4c7e6784b34902b8390771d84ab589ea98fcd3f6.
Common Cargo membership/lock remained root-owned working changes, not this
lane's committed inputs. Earlier fixture path, Clippy and shared-lock mismatch
failures are preserved in private attempts, not erased by final success.

Next dependencies and acceptance:

1. Root resolves each declared Cargo manifest delta; final actual workspace locked
   checks and Git LF bytes must match the committed candidate. Default release
   must exclude the service-pilot fixture binary.
2. Windows runner owns disposable SCM permission delegation to an ordinary actor,
   then runs `cargo test -p humanymous-cli --features service-pilot --locked
   --test service_process -- --ignored --nocapture` with
   HUMANYMOUS_DISPOSABLE_SERVICE_ACTOR=1. Elevated-only execution is blocked,
   not a successful installation. Controller must restore delegated SCM access.
3. Delivery owns actual Linux user-systemd/macOS GUI launchd and six-tuple source-
   bound execution of that same positive pilot. Missing manager/domain is blocked.
4. Runtime owner supplies the agreed ServiceHost private-input/readiness/shutdown
   API; audit owner supplies OS-protected unattended credentials and service-owned
   strict custody children. Only after these connect can actual Core/Gate startup,
   graceful drain, restart and authenticated IPC management be qualified.
5. Release owner supplies independently distributed trust roots, signed release
   descriptors and OIDC/provenance/revocation. A fixture pinned key is not shipped
   release trust or a certificate/notarization result.
6. Faults after partial native unregister, failed old-generation restore, power
   loss or stale journal/lock require explicit owned recovery; failclosed residual
   state is preserved. Automatic crash recovery and full product Docker e2e remain
   open. No whole-program or whole-CLI operational-completion claim is made.

## Handover checkpoint

Accepted follow-up policy is linked through program-roadmap and delivery SoTs,
not reimplemented here. CLI's existing FakeManager/Probe and ACL/temp-file tests
are hosted component checks, not pure local fastchecks. No new local runtime,
service, browser, Docker, broad private-spec scan or heavy research is authorized
by this policy update. Current-source native positive service and six tuples
remain not_run; root owns source closure, dispatch and collection of the first
hosted results. Preserve required gates and historical source-bound results.

CLI code/API checkpoint83397fd5d7289a326b890aa547a8f9e1a04fe03f is committed.
Public acceptance summary: [[evidence:root/20261107053001]]. All17CLI source/Cargo
inputs matched canonical Git bytes; tested27-input manifest retained the exact
working hashes of root/common and native-security inputs pending owner commits.
Raw failures/retries/final source manifests remain in the private CLI bundle.

First read CLI SoT and crates/cli/INTEGRATION.md. Reproduce package tests with
service-pilot, locked Clippy and fmt; inspect the ignored positive service case
instead of counting it as passed. Windows runner owner is preparing the
controller-bootstrap/ordinary-child permission handoff; delivery owns native
six-tuple dispatch. Runtime ServiceHost/protected credential integration remains
the next functional dependency. Pending native teardown/recovery must remain
failclosed. Branch changes/main merge/tags/publication, common Cargo/lock edits,
browser-native CLI and any scoring changes remain out of this lane's scope.


