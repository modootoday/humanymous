---
description: "Gate supports GDPR/PIPA DPO workflows — it does not make you compliant — via pseudonymized identifiers, a tamper-evident audit log, and crypto-shred erasure."
keywords: ["GDPR bot detection","DPO compliance","crypto-shred right-to-erasure","pseudonymized audit log","tamper-evident hash-chained log","RBAC dual-control","PIPA data protection","Ed25519 Signed Tree Head","separation of duties","humanymous Gate"]
---

# Start here: Compliance / DPO

> **Quadrant:** How-to / navigation hub.
> **Audience:** Data-protection officers, privacy counsel, and compliance owners evaluating or operating humanymous Gate.

humanymous Gate is a reference implementation, not a production-hardened build; treat the behaviour described here as the documented design, and validate it against your own deployment.

## What Gate gives you

Gate is built so that its bot-detection processing can be reasoned about under a data-protection regime. Raw identifiers — IP address, JA4, HTTP/2 fingerprint, user agent, SNI, device fingerprint — are never stored in the clear; each is written only as a per-subject-key-derived pseudonym (64-hex, scrypt KDF-stretched), so the audit record is **pseudonymous, not anonymous** (GDPR Recital 26): re-identification remains possible, but only through the identifier vault under dual-control. Every enforcement decision is emitted to an append-only, hash-chained audit log with a per-record HMAC and an Ed25519 Signed Tree Head checkpointed every 32 records and co-signed by an independent local witness, giving you a **tamper-evident (not tamper-proof)** record that an offline verifier can check without trusting the operator. Right-to-erasure is implemented as cryptographic erasure (crypto-shred): destroying the per-subject linkage key renders that subject's identifiers unrecoverable while the chain and Merkle anchors stay intact and verifiable, and a signed erasure certificate is sealed when the shred executes (after the hold window elapses — a commit only schedules it, and an erasure cancelled during the hold window produces no certificate). Administrative access is governed by role-based access control (Auditor, Operator, Approver, DPO) with dual-control on the highest-impact actions — permanent bans, erasure, and the kill switch. Taken together, these features **support your GDPR/PIPA erasure and audit workflows; they do not, on their own, make you compliant.**

> **Warning:** Cryptographic erasure is irreversible. Once the per-subject linkage key is shredded, the pseudonyms in the audit log can no longer be resolved to the subject, and the sealed erasure certificate is the only remaining evidence of that subject's prior records. A cancellable hold window (default 5 minutes) precedes commit; after that window, the shred cannot be undone.

## Your next three reads

1. **[Right-to-Erasure (crypto-shred) Runbook](../runbooks/erasure-crypto-shred.md)** — the DPO-gated, two-person, two-phase procedure for accepting an erasure request, the hold window and its cancellation path, and the signed erasure certificate you retain as evidence. Read this first: it is the workflow you will operate.

2. **[Concepts & Glossary](../concepts/how-gate-sees-a-request.md)** — read the audit-log and pseudonymization sections for the precise meaning of pseudonym derivation, the hash chain, Signed Tree Heads and the independent witness, integrity-mismatch classes, and retention tiers (HOT ~90 days / WARM ~1 year / COLD ~7 years). This is the vocabulary the runbooks and reference pages assume.

3. **[RBAC, separation-of-duties & dual-control reference](../reference/rbac-separation-of-duties.md)** — the role×capability matrix (Auditor / Operator / Approver / DPO), server-derived actor identity, and which actions need a distinct committer (permanent/CIDR bans and the kill switch need a distinct Approver; erasure needs a distinct DPO). This is your separation-of-duties evidence for an audit.

## Verify it yourself

The audit and integrity surfaces are exposed on the authenticated admin listener (default `127.0.0.1:8445` (loopback)), under the admin API base `/__hmn/admin/` with bearer-token auth. Relevant read-only endpoints for a DPO or auditor include `GET /__hmn/admin/integrity`, `GET /__hmn/admin/audit` (filterable by verdict, host, route, rule, minimum risk, and a `before` cursor), and `GET /__hmn/admin/erasures`. Every authenticated access is itself meta-audited before the response is served.

---

See also: [Docs home](../README.md).
