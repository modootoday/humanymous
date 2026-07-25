---
description: "Maps humanymous Gate's reference-build controls to GDPR and PIPA articles and the RFCs they use, plus a repeatable DSAR audit evidence-pack. Controls support your obligations; they do not discharge them."
keywords: ["GDPR bot detection compliance","PIPA destruction obligations","RFC 8032 Ed25519 audit log","RFC 6962 Merkle Signed Tree Head","RFC 7914 scrypt pseudonymization","crypto-shred right to erasure","DSAR audit evidence pack","tamper-evident audit chain","data protection by design","humanymous Gate reference implementation"]
---

# Standards and regulatory mapping matrix, plus audit-response evidence pack

**Diátaxis quadrant:** Reference. **Audience:** Data Protection Officers, compliance and legal reviewers, and procurement / vendor-assessment teams.

This page maps the controls that humanymous Gate ("Gate" hereafter) implements in the reference build to the specific standards and regulatory articles those controls support. It then gives you a repeatable evidence-pack procedure for responding to a supervisory-authority inquiry or a data-subject access request (DSAR).

> **Note:** In your data-processing relationships, you (the operator) are the **data controller**. Gate is **processing tooling** you deploy and configure. This document describes controls Gate **provides** and how they **support** your obligations. Gate does not, and cannot, make you compliant — that determination rests with you and your supervisory authority. Read every "supports" and "provides controls for" below in that frame.

Gate in the reference build is a **reference implementation, not a production-hardened product**. Several anchors that a production deployment would add are called out explicitly as **production responsibility** (not shipped in the reference). See [Production vs reference](./production-vs-reference.md) for the full boundary.

---

## Part 1 — Control-to-requirement traceability matrix

### How to read this table

- **Control** — the mechanism Gate implements.
- **Standard / requirement** — the specific RFC or regulatory article the control supports. Only standards **actually used** in the reference build appear here.
- **What it supports** — the obligation or property the control helps you meet. Bounded language throughout: a control *supports*, *provides controls for*, or *is evidence toward* a requirement; it does not discharge it.
- **Where to verify** — the endpoint, view, or document that lets an assessor independently confirm the control.

### Cryptographic and integrity controls

| Control | Standard / requirement | What it supports | Where to verify |
|---|---|---|---|
| Ed25519 signing of the Signed Tree Head (STH) | RFC 8032 (EdDSA / Ed25519) | Signed, publicly verifiable checkpoints; offline auditors use published keys (`GET /__hmn/admin/keys`). Live Integrity also checks HMAC. | Integrity view / keys / [Verify the audit log](../how-to/verify-audit-log.md) |
| Merkle Signed Tree Head, an STH every 32 records, with independent local witness co-sign | Pattern modeled on RFC 6962 (Certificate Transparency) | Tamper-evident append-only history; the witness co-sign stops silent history rewrites of anchored records. | Integrity view / `GET /__hmn/admin/integrity` |
| scrypt KDF-stretch for subject-key-derived pseudonyms (N=2¹² work factor) | RFC 7914 (scrypt) | Raises the cost of re-identifying pseudonymized identifiers from stored records. | [Data processing inventory](./data-processing-inventory.md) |
| scrypt sealing of the keystore (N=2¹⁵ work factor) with AES-256-GCM | RFC 7914 (scrypt); AES-256-GCM (AEAD) | Confidentiality and integrity of sealed key material (Ed25519 STH signing seed, HMAC key, vault snapshot) at rest. | [Key management](../how-to/key-management.md) |
| Per-record HMAC over the audit chain | AES-256-GCM keystore protects the HMAC key; supports Art. 32 (see below) | Per-record integrity in addition to chain linkage. | Integrity view; mismatch class `hmac-invalid` |

> **Note:** Work factors above are transcribed as `N=2^12` (pseudonym stretch) and `N=2^15` (keystore seal). The exact SPDX license of each third-party Go dependency is **not asserted here** — confirm each from that module's own LICENSE file before publishing a vendor bill-of-materials.

> **Note:** humanymous Gate is released under the **Apache License 2.0** (`LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.md` at the repository root). See [Support, licensing & open-source notices](support-licensing.md) for the full statement and the third-party dependency bill-of-materials.

### GDPR controls

| Control | Standard / requirement | What it supports | Where to verify |
|---|---|---|---|
| Raw identifiers (IP, JA4, HTTP/2 fingerprint, UA, SNI, device fingerprint) stored **only** as per-subject-key-derived pseudonyms (64-hex, scrypt-stretched) | GDPR Art. 4(5) + Recital 26 (pseudonymization) | Pseudonymization by default; identifiers are pseudonymous, **not anonymous** — re-identification is offline for keystore holders (no product dual-control API). | [Data processing inventory](./data-processing-inventory.md) |
| Data-protection-by-design defaults: pseudonymize-at-write, fail-closed edge on strict/unsafe requests, least-privilege role-based access control | GDPR Art. 25 (data protection by design and by default) | Controls that support privacy-preserving defaults rather than opt-in configuration. | [role-based access control and separation of duties](./rbac-separation-of-duties.md) |
| Purpose-scoped, minimized processing recorded in a Records-of-Processing extract | GDPR Art. 5 (principles) | Evidence toward lawfulness, purpose limitation, data minimization, and storage limitation (retention tiers). | [Data processing inventory](./data-processing-inventory.md) |
| Cryptographic erasure (crypto-shred): destroy the per-subject linkage key; chain intact; `erasure.completed` audit record on **execution** (not a standalone signed certificate blob; cancelled erasures produce none) | GDPR Art. 17 (right to erasure) | A repeatable, verifiable erasure mechanism that preserves audit integrity while severing re-identifiability for that session subject. | [Erasure and crypto-shred runbook](../runbooks/erasure-crypto-shred.md) |
| Records-of-Processing content to assemble a RoPA manually | GDPR Art. 30 (records of processing activities) | Provides the processing-activity content you assemble into your RoPA. **No machine-readable RoPA export ships** — the inventory is documentation you transcribe, not an exportable artifact. | [Data processing inventory](./data-processing-inventory.md) |
| Tamper-evident hash chain + per-record HMAC + Ed25519 STH; role-based access control; dual-control on high-impact actions; meta-audit of every admin access | GDPR Art. 32 (security of processing) | Integrity, access control, and accountability controls that support security of processing. | Integrity view; [role-based access control and separation of duties](./rbac-separation-of-duties.md) |
| DPIA companion content covering the processing, risks, and mitigations | GDPR Art. 35 (data protection impact assessment) | Structured input you adapt into your own DPIA. | [DPIA companion](../explanation/dpia-companion.md) |

### PIPA (Korea)

| Control | Standard / requirement | What it supports | Where to verify |
|---|---|---|---|
| Crypto-shred after dual-control hold; `erasure.completed` audit record (HMAC-chained; next STH covers it); HOT/WARM/COLD labels declared only | PIPA destruction obligations | Verifiable destruction of the linkage that makes a **session subject** re-identifiable; retention lifecycle is operator-owned. | [Erasure and crypto-shred runbook](../runbooks/erasure-crypto-shred.md) |
| Append-only audit chain, admin-action meta-audit, integrity verification | PIPA records / access-management obligations | Access and processing records that support your record-keeping duties. | `GET /__hmn/admin/audit`; Integrity view |

> **Warning:** Cryptographic erasure (crypto-shred) is **irreversible**. It destroys the per-subject linkage key so the affected records can no longer be re-identified. The records themselves are never deleted and remain in the verifiable chain, but once the key is shredded the pseudonym-to-subject link cannot be recovered. There is a cancellable hold window (default 5 minutes) before the shred executes; after it commits there is no recovery path.

### Recommended production anchors — NOT shipped in the reference

The following are established anchors a production deployment would add. They are **production responsibility**: they are **not implemented** in the reference build. Do not represent them as shipped features in a vendor assessment.

| Recommended anchor | Standard | Status |
|---|---|---|
| Trusted timestamps on STH checkpoints | RFC 3161 (Time-Stamp Protocol) | **production responsibility — not shipped.** Recommended production anchor for externally attestable checkpoint time. |
| External WORM / immutable object storage for chain and checkpoints | S3 Object-Lock / WORM retention | **production responsibility — not shipped.** Recommended production anchor for off-node immutability. |

> **Note:** Because RFC 3161 timestamping and external WORM anchoring are not shipped, the reference audit log is **tamper-evident, not tamper-proof**: records written after the last checkpoint remain re-writable until the next checkpoint anchors them (an unanchored in-window residual). The independent witness co-sign stops silent history rewrites of already-anchored records. Present this boundary honestly in any assessment.

---

## Part 2 — Audit-response evidence pack

This checklist turns a supervisory-authority inquiry or a DSAR into a repeatable procedure. Each item names the artifact, how to produce it, and how the recipient can independently verify it. All admin endpoints are served on the **admin listener** (default `127.0.0.1:8445` (loopback)) under the `/__hmn/admin` base, behind bearer authentication; the public edge returns 404 for these paths.

Throughout, replace `<admin-host>` (default `localhost:8445`), `<token>` (an admin bearer token with the required role), and other `<angle-bracket>` placeholders with your values.

### Evidence-pack checklist

- [ ] **RoPA extract** — Records of Processing (Art. 30 / PIPA records)
- [ ] **DPIA** — impact assessment adapted from the companion (Art. 35)
- [ ] **Erasure certificates and their verification** — Art. 17 / PIPA destruction
- [ ] **Integrity reports** — audit-chain verification (Art. 32)
- [ ] **Audit export** — filtered decisions and CSV (Art. 30 / Art. 32)
- [ ] **role-based access control and dual-control configuration** — separation of duties (Art. 25 / Art. 32)
- [ ] **Admin-action meta-audit trail** — accountability (Art. 32)

### 1. RoPA extract

Assemble your Records of Processing from the [data processing inventory](./data-processing-inventory.md). It enumerates the categories of identifiers processed, how each is pseudonymized, the retention tiers (HOT ≈ 90 days / WARM ≈ 1 year / COLD ≈ 7 years), and the lawful-basis framing. You (the controller) sign off on the final RoPA; the inventory provides the processing-activity content.

### 2. DPIA

Adapt the [DPIA companion](../explanation/dpia-companion.md) into your own assessment. It documents the processing purpose, the risk to data subjects (including false-positive risk and the residual attacker-tier ceiling), and the mitigations Gate provides. The DPIA is your document; the companion is structured input.

### 3. Erasure certificates and their verification

For a right-to-erasure request, follow the [erasure and crypto-shred runbook](../runbooks/erasure-crypto-shred.md). The certificate is sealed on **execution** — when the cancellable hold window elapses and the shred actually runs — **not** on commit; a cancelled erasure produces **no** certificate.

There is **no admin endpoint that returns the certificate** (production responsibility). The `/__hmn/admin/erasures` endpoint lists only erasures still **within** their hold window (scheduled but not yet executed); once the shred runs, the certificate is observable only as an `erasure.completed` **audit record** in the stream:

```
# List scheduled (in-hold-window) erasures only:
curl -k -H "Authorization: Bearer <token>" \
  https://<admin-host>/__hmn/admin/erasures

# The completed-erasure certificate is an audit record — retrieve it from the audit stream:
curl -k -H "Authorization: Bearer <token>" \
  "https://<admin-host>/__hmn/admin/audit" \
  | jq '.records[] | select(.event_type == "erasure.completed")'
```

The certificate record is a plain audit record **HMAC-chained** into the log; it is **not independently Ed25519-signed at seal time**. It is anchored — and becomes tamper-evident — only when the **next Ed25519 STH checkpoint** covers it. So the certificate is verifiable as part of the audit chain (via the next checkpoint, public key alone), not as a standalone signature you verify in isolation. The chain and Merkle anchors remain intact after a shred, which the Integrity view confirms independently (see step 4).

> **Warning:** Do not initiate an erasure to produce a demonstration certificate. Crypto-shred is irreversible; the affected subject's records become permanently non-re-identifiable once the hold window elapses and the shred commits.

### 4. Integrity reports

Produce a current integrity report over the audit chain:

```
curl -k -H "Authorization: Bearer <token>" \
  https://<admin-host>/__hmn/admin/integrity
```

The report verifies the hash chain, per-record HMAC (when a key is supplied), and Ed25519-signed checkpoints, and reports any mismatch by class: `hash-break`, `hmac-invalid`, `hmac-unchecked`, `empty-chain`, `seq-gap`, `linkage-break`, `checkpoint-mismatch`, `witness-invalid`, or `node-missing`. Assessors with published public keys re-verify offline; live Integrity uses the node key material. See [Verify the audit log](../how-to/verify-audit-log.md).

### 5. Audit export

Export the decision records relevant to the inquiry. The audit endpoint supports server-side filters and a cursor:

```
curl -k -H "Authorization: Bearer <token>" \
  "https://<admin-host>/__hmn/admin/audit?verdict=<ALLOW|CHALLENGE|DENY>&host=<host>&route=<route>&rule=<HR-id>&minRisk=<0-100>&before=<cursor>"
```

Page through results with the returned `nextBefore` cursor. Each record carries the verdict, risk score, matched enforcement rule (if any), and the pseudonymized identifiers — never raw identifiers.

For a human-readable snapshot, the Ledger Overview view has an **Export CSV** button that downloads `audit-feed.csv` (`text/csv`). See the [Ledger tour](../how-to/audit-console-tour.md).

> **Note:** Observability in the reference build is the audit stream, the Integrity view/endpoint, the Overview KPIs, Prometheus `GET /__hmn/admin/metrics` on the admin plane, and edge `GET /__hmn/healthz` / `GET /__hmn/readyz`. SIEM shipping of the audit stream is still a production responsibility.

### 6. role-based access control and dual-control configuration

Provide the effective policy, whose `config_version` is a signed HMAC hash of the effective policy (routes, rate, monitor, kill switch):

```
curl -k -H "Authorization: Bearer <token>" \
  https://<admin-host>/__hmn/admin/policy
```

Pair this with [role-based access control and separation of duties](./rbac-separation-of-duties.md), which documents the four roles and their capabilities:

- **Auditor** — read only.
- **Operator** — read + operate (temp bans, lifts).
- **Approver** — read + approve.
- **data protection officer** — read + operate + approve, and the **only** role that can approve erasure.

Dual-control facts to state in the pack: a permanent or CIDR ban and the kill switch must be committed by a **distinct Approver**; an **erasure must be committed by a distinct data protection officer** (a generic Approver cannot). Temporary bans and lifts are single-Operator.

### 7. Admin-action meta-audit trail

Every admin access is meta-audited under `admin.access`, with the actor identity **derived server-side** (any actor field in a request body is ignored). Filter the audit export for these records to produce the accountability trail:

```
curl -k -H "Authorization: Bearer <token>" \
  "https://<admin-host>/__hmn/admin/audit?route=control" \
  | jq '.records[] | select(.event_type == "admin.access")'
```

The audit export has no event-type query parameter (its server-side filters are `verdict`, `host`, `route`, `minRisk`, `before`, `limit` and `rule`). The `rule` filter matches the `triggered_rules` slice, not the event taxonomy, so `?rule=admin.access` selects nothing. Meta-audit records carry `route_class: "control"`, so `?route=control` narrows to the control plane server-side; isolate `admin.access` precisely by filtering the exported `event_type` field client-side, as shown.

---

## Scope and boundaries to state plainly

- You are the controller; Gate is processing tooling. These controls **support** your obligations; they do not discharge them.
- The reference build is a **reference implementation, not production-hardened**. RFC 3161 timestamping and external WORM anchoring are recommended production anchors, **not shipped**.
- The audit log is **tamper-evident, not tamper-proof** (unanchored in-window residual; witness co-sign stops silent rewrites of anchored history).
- Pseudonymized identifiers are **pseudonymous, not anonymous** (GDPR Recital 26).
- Cryptographic erasure is **irreversible**.

## Related references

- [Data processing inventory](./data-processing-inventory.md) — RoPA source content (Art. 30)
- [DPIA companion](../explanation/dpia-companion.md) — DPIA input (Art. 35)
- [Verify the audit log](../how-to/verify-audit-log.md) — integrity verification procedure
- [role-based access control and separation of duties](./rbac-separation-of-duties.md) — roles, capabilities, dual-control
- [Erasure and crypto-shred runbook](../runbooks/erasure-crypto-shred.md) — right-to-erasure procedure
- [enforcement rules and verdicts](./hard-rules-verdicts.md) — verdict and enforcement-rule glossary
- [Production vs reference](./production-vs-reference.md) — the full production responsibility boundary
