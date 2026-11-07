# Readiness, incidents and recovery

Establish the owned deployment, installed source/artifact identity, native service
identity and affected request path. Prefer sanitized diagnostic status and bounded
counts over raw client headers, addresses, bodies or credentials. Do not read a
browser profile or export keys to diagnose a service problem.

`service-doctor --destination PRIVATE_ROOT --trust-key PINNED_PUBLIC_KEY_HEX`
checks the existing deployment and its diagnostic readiness. It accepts neither
`--activate` nor `--dry-run`. A native-manager running state plus loopback readiness
is a diagnostic result, not authorization, authenticated IPC proof, collector
qualification or full product health. A successful `doctor` does not grant
permission to restart or change services.

`service-rollback` and `service-remove` accept destination/trust-key and support
`--dry-run`; mutation requires explicit `--activate`. Signed-service rollback
selects the known signed predecessor and retains its high-water sequence. It is
different from provisioning-only `rollback`. Inspect the intended effect before
using either. No arbitrary service deletion or unverified binary replacement.

On missing authority, timeout, invalid peer/ACL, malformed state, signature/digest
failure or replay, preserve refusal and inspect the relevant producer/transport/
credential/readiness boundary. Do not switch to ALLOW, spoof a peer via headers,
weaken a required signal, disable the sandbox or retry through an unapproved
transport. A timeout/dependency error is not a new detection verdict.

Keep audit/keys/witness custody and user-data handling intact during restart or
rollback. Unknown configuration, a pending journal or cleanup/restore failure
requires explicit recovery; do not erase evidence or silently recreate identity.
No single operator can authorize erasure, permanent bans or kill-switch effects
where dual control is required.

Conclude with observed source, command/action, actual diagnostic result, user
effects, remaining unknowns and the next owner. Actual native-manager/IPC/runtime
pilots belong in the appropriate authorized environment; ordinary mock/unit
success cannot qualify those boundaries.
