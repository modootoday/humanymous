---
id: plan:root/20261107211302
title: Accepted first public role data schema slice
status: active
created: 2026-11-07T21:13:02Z
updated: 2026-11-07T21:13:02Z
rel:
  depends_on: [sot:root/structural-development, sot:root/program-roadmap, plan:root/20261107205157, plan:root/20261107205817, plan:root/20261107205702]
---

# Scope and decision

Root accepts the first pure Runtime public-role DATA parser as an implementation
slice. This is not approval of a public listener, origin resolver, provider,
certificate issuance, native preparation, witness receipt or Ready producer.
Existing private-loopback Configuration v1 remains unchanged. All27 shipping
behavior families remain required; this small slice does not replace them.

Runtime owns the one schema and parser, reused by CLI later. Author under the
current Runtime shipping context; its eventual destination is gateway::shipping
inside the accepted runtime package. No new package, duplicated crypto/parser,
production testing-feature dependency or Cargo dependency change is needed.

# Exact accepted wire

Maximum original input is32768 bytes, enforced before parsing. Root and all nested
objects are closed; every listed field is required. Refuse duplicate decoded
keys, unknown/missing fields, aliases, trailing JSON and wrong scalar types.
Standard JSON whitespace and escapes remain valid; no canonical-JSON claim.

| Field | Accepted type and bounds |
| --- | --- |
| schema | Exact ASCII humanymous.public-role/1 |
| role | Exact enum core_demo or gate_edge |
| scope | Closed object with installation, tenant, origin, revision and application_authority |
| scope installation/tenant/origin/revision | Each1..128 decoded ASCII characters from A-Z, a-z,0-9, period, underscore, colon and hyphen |
| scope application_authority |1..1024 decoded ASCII bytes33..126 excluding at-sign, slash, hash, question mark, backslash and comma; exact comparison, no normalization; a DATA token, not address validation |
| binding | Closed object with installation_epoch, generation, base_plan_sha256 and executable_sha256 |
| binding installation_epoch | Exactly32 lowercase hexadecimal characters decoded to16 bytes; reject all-zero epoch |
| binding generation | Positive JSON u64 integer, at most9007199254740991; no fractional, negative or overflow representation |
| binding digests | Each exactly64 lowercase hexadecimal characters decoded to32 bytes; grammar alone does not verify an artifact |
| required_providers | Closed enum vector, at most6 entries during visiting, no duplicates |

Provider enum values are durable_audit, external_witness, native_preparation,
original_tls, original_h2 and collector. Both public roles must declare the first
four. H2 and collector declarations express additional required DATA; omitting
them cannot waive a later role/route/ALPN capability requirement. Provider
declaration is never observation of provider availability or successful admission.
Reject a seventh element before collecting an unbounded vector.

base_plan_sha256 commits to an external original document, never this envelope
or a reserialized copy. Epoch/generation and digests remain caller assertions.
Parsing does not reserve a generation, reject a real replay, check freshness or
verify executable/source/native ownership. Do not add a binding-comparison helper
in this first slice; future protected-input binding receives its own contract.

# API and tests

Expose one pure parse_public_role_data byte-slice entrypoint returning immutable
PublicRoleData views or a fixed redacted error. No raw serde/foreign error or
input echo; no arbitrary map/extensions, embedded document, secret, address/path,
certificate/credential reference, clock, callback, filesystem, DNS or network I/O.
Do not derive a raw-data Debug/serialization surface merely for diagnostics.

Meaningful units cover both valid roles; exact boundary lengths and raw budget;
decoded duplicate/escaped keys at every object level; each unknown/missing/type
case; generation boundaries; epoch/digest grammar; provider mandatory membership,
duplicates/unknown enum/seventh-entry refusal; and redacted errors. Preserve the
original caller bytes and distinguish syntax-valid DATA from any authority.

Author code and tests now under the existing Runtime lane after claiming it.
No compilation starts until Root allocates a source-frozen bounded window. Record
exact paths/hashes, review and unexecuted gates in the owning spec. Public ingress,
endpoint/preset/origin/admin schemas, container/native/W/custody, shutdown budgets,
TLS/H2 and actual image roles remain separate pending decisions and implementations.
