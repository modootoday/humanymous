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
API. The optional runtime-service feature now connects that shared ServiceHost,
StopBudget, typed run_report result, native stop and an independent watchdog.
`humanymous runtime-host --plan PATH --plan-sha256 SHA256` admits only the strict
nonsecret host-plan. Its exact digest belongs in the signed release arguments.
On Windows the plan also binds installer_owner_sid; the actual virtual service
principal must verify the original owner and exact delegated read/execute role.
Its runtime configuration must be current-service owner-only, digest-pinned and
contain an empty passphrase. NativeSecrets consumes only shared native-security
service credential reads; no passphrase is accepted through argv/env/plain files.

Root reported initial runtime-service compile success with three warnings; those
source warnings were fixed before further accounting/input changes. Current
source has seven pure runtime_host tests and direct rustfmt passed, but current
Cargo/Clippy and actual lifecycle require source-bound root/hosted results. Do not
run the entire CLI lib harness as a local pure check: existing service tests touch
real files, ACLs and ports. The scoped pure filter is `--features runtime-service
--lib runtime_host::` under an explicit shared Cargo slot.

The separate runtime-service-pilot feature enables only the shared disposable
credential fixture and optional audit dependency; no production feature enables
credential writes/deletion. The actual runtime SCM fixture now connects the shared
create-new protected-directory APIs and remains NOT_RUN. Its ignored hosted test
is `runtime_service_process::native_runtime_service_lifecycle_and_missing_credential_fail_closed`
(Cargo target `--test runtime_service_process`, exact test name without module
prefix). Run with `--features runtime-service-pilot --ignored --exact` only in the
disposable hosted actor. Controller metadata HUMANYMOUS_SERVICE_PILOT_RUN_ID and
HUMANYMOUS_SERVICE_PILOT_SOURCE_SHA is copied into pinned nonsecret launch input;
the service never assumes job environment inheritance. The parent independently
correlates both actual SCM PIDs/statuses with bounded receipt fields and prints
exactly one aggregate `humanymous.runtime-service-pilot/1` JSON line. Private raw
receipts contain service PID; public aggregate has only booleans/source/run/case.
Linux/macOS protected-secret
providers currently return Unsupported. Fresh authenticated observer provisioning,
trusted external-witness production input, recoverable Windows typed failure
transport, full Core/Gate runtime qualification and Docker product e2e remain open,
alongside release trust/OIDC/provenance/revocation. Fixture metadata is not hosted
attestation or a product credential.
