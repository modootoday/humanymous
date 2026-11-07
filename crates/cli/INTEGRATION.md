# CLI integration

Provisioning binary: `humanymous`, separate from browser-owned native-cli framework
tooling. Rust1.99.1/edition2024; publish=false. Default builds exclude the
`humanymous-service-fixture` binary behind `service-pilot`.

Legacy doctor/install/rollback retain explicit SHA256, operator-locality
acknowledgement, private parent, exact repeat and managed-file rollback semantics.
New service-plan/install/upgrade/doctor/rollback/remove verify offline pinned-key
Ed25519 descriptors, signed host/digest/name/argv/readiness, immutable generations,
private state and sequence floor. See CLI SoT for exact schema and limits.
Mutations require `--activate`; dry-run has no write/lock/probe.

Service install/upgrade arguments: `--manifest PATH --signature LOWERCASE_HEX
--trust-key PINNED_PUBLIC_KEY_HEX --artifact PATH --destination PRIVATE_ROOT`.
Existing service doctor/rollback/remove accept destination/key only. Rollback and
remove also accept `--dry-run`; doctor queries actual manager and loopback
diagnostic readiness. Success JSON always retains `product_qualified=false`.

Backends: Linux runtime-only systemd user link/start, macOS user launchd bootstrap,
Windows demand-start own-process virtual service account. Only owned
`humanymous-lab-` names are supported in this reference boundary. No elevation,
host user creation, password, autostart or arbitrary service adoption. Unknown
native config/content/ACL, replay, collision, pending journal or stale lock fails
closed. Registration/readiness failure cleans the new generation and restores
the predecessor. Cleanup/restore failure preserves pending state; crash-atomic
or power-loss recovery is not claimed.

Windows reuses shared native-security APIs from26d9925: resolve_service_sid,
grant/revoke_service_access, verify_service_access and ServiceRole. Strict
installer/audit/custody profiles remain unchanged. New service generations use
artifact.exe; legacy provisioning retains artifact. Stage receipts explicitly
say local-management=not-runtime-qualified; deployment.json owns service state.
A StateDirectory activation grant is never strict custody: runtime must create
its own protected service-owned child for keys/audit.

CLI adds existing shared ed25519-dalek/hex/serde/serde_json and pinned Windows
windows-service0.8.1; root owns common Cargo/lock integration. Native API/ACL logic
is shared; no authored shell or executable-script string is used.

Validation: `cargo test -p humanymous-cli --features service-pilot --locked
-- --nocapture`, warnings-denied package Clippy with all-targets/service-pilot,
and package fmt. `cargo run -p humanymous-cli --example service_evidence --locked`
emits working-source hashes and separately observed canonical Git byte hashes.
It is provenance, not product qualification. Exact final counts/source belong
to CLI evidence; mocks and actual process cases remain separate.

Positive native lifecycle pilot requires HUMANYMOUS_DISPOSABLE_SERVICE_ACTOR=1
and `cargo test -p humanymous-cli --features service-pilot --locked --test
service_process -- --ignored --nocapture`. It registers/starts/probes, checks
repeat, upgrades, restores predecessor, rejects replay and removes owned changes.
JSON schema is humanymous.cli-service-pilot.v1. Without --ignored, Windows runs
two actual process/refusal cases and preserves one ignored positive case.

Windows runner must temporarily delegate SCM CONNECT|CREATE_SERVICE to an
ordinary dedicated actor in a disposable VM and restore the original manager
ACL. Elevated direct execution intentionally fails installer policy. Local host
escalation/SCM permission changes were not performed. Delivery owns Linux/macOS
real manager and six-tuple pilots; missing manager/domain is blocked. Positive
native-service results have not been observed locally.

Runtime owner accepted the proposed ServiceHost private-input/readiness/shutdown
API, not yet connected here. Audit owner supplies protected unattended credentials
and service-owned strict custody children. No config/passphrase is invented in
argv/env/plaintext files. Fixtures have no product credentials. Full Core/Gate
startup, authenticated IPC/local management, graceful runtime drain and Docker
product e2e remain open, alongside release trust/OIDC/provenance/revocation.
