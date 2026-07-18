# DPIA companion pack and retention/lifecycle guide

> **Diátaxis quadrant:** Explanation. **Audience:** Data Protection Officers (DPOs), privacy counsel, and supervisory-authority reviewers.

This page is a compliance companion for a Data Protection Impact Assessment (Article 35 GDPR) covering an operator deployment of humanymous Sentinel. It is written as a **risk-and-mitigation register**, not a set of assurances. Each processing property below is stated first as the residual risk it carries, then as the mitigation the reference implementation provides, and — where the mitigation is incomplete or deferred — as an explicit limit the controller must accept or close.

humanymous Sentinel ("Sentinel") is a reverse-proxy enforcement layer that terminates TLS, streams a detection bundle into HTML responses, scores each request across layers L1–L7, enforces a verdict (ALLOW / CHALLENGE / DENY) at the edge, and writes every decision to a tamper-evident audit log. This document does not repeat the field-level processing register; read it alongside the [data-processing inventory](../reference/data-processing-inventory.md), the [standards mapping](../reference/standards-mapping.md), the [erasure (crypto-shred) runbook](../runbooks/erasure-crypto-shred.md), and [RBAC and separation of duties](../reference/rbac-separation-of-duties.md).

> **Important:** This document describes a **reference implementation**, not a production-hardened product. Several controls a DPIA would rely on in production are prod-deltas (identified inline). Sentinel is processing **tooling**; determining lawful basis, necessity, proportionality, and the DPIA conclusion for a specific deployment remains the responsibility of the operator acting as **data controller**.

---

## 1. Controller/processor framing

The operator who deploys Sentinel in front of an origin application is the **data controller** for the traffic it inspects. Sentinel is the processing tooling the controller configures and operates; it fronts the origin and does not control it. Sentinel does not transmit personal data to humanymous or any third party — all processing described here is in-process on infrastructure the operator runs.

The controller is accountable for the Article 30 record of processing activities, the Article 35 DPIA itself, the lawful-basis determination, data-subject communications, and the retention schedule. This companion supplies the technical facts those instruments require; it does not substitute for them.

> **TODO(verify):** The Article 6 lawful basis for bot-detection processing (for example, legitimate interests under Art. 6(1)(f) with a balancing test, or another basis) is a controller determination and is not fixed by the reference implementation. State the chosen basis and, where legitimate interests is relied on, attach the legitimate-interests assessment.

---

## 2. Purpose, necessity, and proportionality

### 2.1 Purpose

The processing purpose is **distinguishing automated clients from human users** at the request boundary, so the controller can block, challenge, or allow traffic before it reaches the origin application. The detection covers non-browser clients, naive and stealth-patched automation frameworks, real-engine automation, and — as a design boundary, not a solved case — anti-detect tooling combined with human click-farms.

### 2.2 Necessity

Necessity turns on whether the purpose can be met with less personal-data processing. The signals Sentinel evaluates (static client attributes, fingerprint material, client-integrity checks, behavioral timing, and network/protocol characteristics such as TLS and HTTP/2 fingerprints and header ordering) are the inputs that let the engine separate automation from humans across layers L1–L7. A cross-check-first, defense-in-depth design is used specifically so that no single identifier is decisive and low-false-positive verdicts can be reached deterministically.

The proportionality-relevant design choices in the reference are:

- **Data minimization by pseudonymization at rest.** Raw identifiers are never stored in the audit log in clear (Section 3).
- **Monitor mode.** The controller can run detection without enforcement, observing verdicts before any traffic is blocked or challenged — allowing the necessity and false-positive profile to be measured on the controller's own deployment before enforcement is turned on.
- **Route-scoped strictness.** Enforcement strength is set per route at startup (for example, stricter defaults on sign-in and checkout paths), rather than applying the maximum posture uniformly.

### 2.3 Proportionality and the false-positive residual

Proportionality must account for the impact on **legitimate human users** who are wrongly challenged. The reference is designed for low false positives, and its hard rules are tiered by confidence: several are near-zero-false-positive automation tells, while at least one heuristic rule (no interaction over a window) can catch some humans and therefore issues a CHALLENGE rather than a DENY. The proportionate posture is that lower-confidence signals **challenge** (a recoverable proof-of-work interstitial) rather than **block**. The controller should document the residual risk that a human is challenged, and the accessibility of that challenge, as part of the balancing test. See also [Will this break my app?](will-this-break-my-app.md).

> **Note:** The reference challenge is a minimal interstitial. A full WCAG 2.2 AA challenge experience is a prod-delta and an operator responsibility; the accessibility of the production challenge is a proportionality input the DPIA should record.

---

## 3. Pseudonymization design and its limits

### 3.1 Design

Raw identifiers — IP address, JA4/TLS fingerprint, HTTP/2 fingerprint, User-Agent, SNI, and device fingerprint — are **not** stored in the audit log in clear. Each is stored only as a **per-subject-key-derived pseudonym**: a 64-hex value produced by an scrypt key-derivation stretch (N = 2^12), where the subject key is the session identifier. This is pseudonymization within the meaning of Article 4(5) GDPR and Recital 26.

**Risk addressed:** An adversary (or an over-broad internal read) who obtains the audit log alone does not obtain the underlying identifiers in usable form, and cannot trivially correlate records across subjects, because the derivation is keyed per subject.

### 3.2 Limit — pseudonymous is not anonymous

Pseudonymized records remain **personal data** (Article 4(5), Recital 26). They are re-identifiable by design when combined with the keying material. The DPIA must treat the audit log as personal data throughout its lifecycle, not as anonymized data outside GDPR scope.

### 3.3 Limit — low-entropy identifiers remain re-derivable if the subject key leaks

Pseudonymization protects against exposure of the **stored** value, not against a **brute-force pre-image search when the subject key is known**. Because the key derivation is keyed by the subject (session) key, an attacker who obtains that key — or the sealed keystore that protects it — can re-derive the pseudonym for a **guessed** identifier and confirm a match. For a high-entropy identifier this is impractical; for a **low-entropy identifier such as an IPv4 address**, the guess space is small enough that a match is confirmable. In other words: the pseudonym does not add meaningful entropy to an already-low-entropy input against an adversary who holds the key.

**Mitigations:**

- The keying material and vault are held in a keystore sealed with scrypt (N = 2^15) and AES-256-GCM (see Section 6). The scrypt stretch raises the cost of offline attack on the sealed material.
- **Cryptographic erasure (crypto-shred)** — destroying the per-subject linkage key — destroys exactly the key that makes re-derivation possible. After crypto-shred, the low-entropy re-derivation path in this section no longer applies to that subject, because the key required to derive or confirm the pseudonym no longer exists. Records remain in the chain but are no longer linkable to the subject (Section 5, Section 7).
- Re-identification through the intended path (the vault) is itself gated and audited (Section 4).

> **Warning:** Cryptographic erasure ("crypto-shred") is **irreversible**. Destroying the per-subject linkage key permanently removes the ability to re-identify that subject's records; the records themselves are retained and remain integrity-verifiable, but the linkage cannot be restored. Loss of the keystore unseal secret (`HMN_UNSEAL`) is functionally equivalent to a mass crypto-shred of all subjects, because the sealed identity material can no longer be opened. Back the unseal secret up out-of-band.

---

## 4. Re-identification vault and the necessity test

Re-identification — turning a stored pseudonym back into the underlying identifier — is not an ambient capability. It **requires the vault plus dual-control**, and the act of re-identification is itself audited.

- **Necessity test.** Because re-identification requires the vault and a second authorizing role, each re-identification is a discrete, gated event, not a routine read. The controller can therefore demonstrate that re-identification occurs only when a specific, recorded need arises.
- **Dual-control.** The re-identification path requires a distinct second actor (name the second role in the authorization record; see [RBAC and separation of duties](../reference/rbac-separation-of-duties.md)).
- **Meta-audit.** The re-identification action is written to the audit log like any other privileged action, producing an accountability trail (Article 5(2), Article 30).

**Residual risk:** An actor who holds both the vault and can satisfy dual-control could re-identify without an externally-justified need. The mitigation is organizational: separation of duties, deny-by-default admin access, and the meta-audit trail that makes such an action reviewable after the fact. The DPIA should record who holds vault access and how the second-role authorization is controlled.

---

## 5. Tamper-evident, not tamper-proof — the in-window residual

### 5.1 Property

The audit log is an **append-only hash chain** with a **per-record HMAC**, anchored by an **Ed25519 Signed Tree Head (STH)** issued every 32 records (a Merkle checkpoint pattern), and **co-signed by an independent local witness**. It is **tamper-evident**: alteration is detectable on verification. It is **not tamper-proof**: the term "tamper-proof" is not claimed.

### 5.2 Risk — the unanchored in-window residual

Records written **after the last signed checkpoint** and before the next are **re-writable until the next checkpoint anchors them**. Within that window there is a residual in which history could, in principle, be rewritten before it is sealed.

### 5.3 Mitigations

- **The independent witness co-sign** is the specific control that stops **silent** history rewrites: because an independent witness co-signs the tree head, a rewrite cannot be anchored without the witness's participation, so silent rewriting of already-checkpointed history is prevented.
- **Frequent checkpoints** (every 32 records) bound the size of the unanchored window.
- **Public verifiability.** Chain integrity verifies with the public key alone, via the Integrity view and the integrity endpoint, so the controller (and, where appropriate, an auditor) can independently confirm the chain has not been altered. Verification distinguishes integrity-failure classes: hash-break, hmac-invalid, seq-gap, linkage-break, checkpoint-mismatch, and node-missing.

**Residual accepted:** The in-window (post-last-checkpoint) residual is an accepted, documented limitation of the reference. The DPIA should record it as a known integrity limit mitigated by frequent anchoring and witness co-signing, not as a solved property.

> **Note:** A standalone offline verifier **process**, RFC 3161 trusted timestamps, and external WORM/object-lock anchoring are prod-deltas (Section 7.3). They are recommended production anchors, not shipped features of the reference.

---

## 6. Safeguards summary (technical and organizational measures, Article 32)

The following measures are what the DPIA can cite as the reference's technical and organizational controls on the administrative plane:

- **RBAC with four roles** — Auditor, Operator, Approver, DPO — with capabilities scoped per role (see [RBAC and separation of duties](../reference/rbac-separation-of-duties.md)).
- **Separation of duties.** Read, operate, approve, and DPO-erasure capabilities are held by distinct roles; no single role holds all.
- **Dual-control** on the highest-impact actions: permanent/CIDR bans and the kill switch must be committed by a **distinct Approver**; **erasure** must be committed by a **distinct DPO** (a generic Approver cannot approve erasure).
- **Meta-audited reads and actions.** Administrative access and privileged actions are written to the audit log; the actor identity is **server-derived**, and any actor value supplied in a request body is ignored — so the accountability trail cannot be spoofed by the caller.
- **Deny-by-default admin plane.** The admin API runs on a separate listener; the console and admin routes return 404 on the public edge, and missing or invalid bearer credentials return 404 rather than a distinguishable auth error. Bearer comparison is constant-time.
- **Sealed keystore.** The signing seed (Ed25519 STH key), HMAC key, and vault snapshot are sealed with scrypt (N = 2^15) and AES-256-GCM under the operator-held unseal secret.

> **Warning:** The **kill switch is fleet-wide**. Activating it demotes hard-rule enforcement to monitor mode across the entire fleet — detection stops and traffic flows to the origin — while manual bans continue to enforce. Its scope and dual-control (a distinct Approver) must be understood before use; record it in the DPIA as a control with fleet-wide blast radius.

> **Note:** In the reference, admin authentication is bearer tokens with dev defaults; real KMS/HSM key custody, ACME certificates, and mTLS/SSO admin authentication are prod-deltas. The DPIA should record the production key-custody and admin-auth model the operator will actually deploy.

---

## 7. Retention and data lifecycle

### 7.1 Retention tiers

A classifier assigns audit records to retention tiers:

| Tier | Approximate retention |
| --- | --- |
| HOT | ~90 days |
| WARM | ~1 year |
| COLD | ~7 years |

These are the reference defaults. The controller must reconcile them with its own retention schedule and legal obligations, including Korea's PIPA destruction and record-keeping obligations where applicable, and document the justification for each tier under the storage-limitation principle (Article 5(1)(e)).

> **TODO(verify):** The classifier's tier-assignment rules (which record types map to HOT/WARM/COLD, and the exact boundary durations beyond the approximate values above) are not enumerated in the available facts. Confirm the mapping before asserting a specific per-record retention period.

### 7.2 Why crypto-shred rather than deletion — WORM compatibility

The right-to-erasure mechanism (Article 17) is **cryptographic erasure (crypto-shred)**: the per-subject linkage key is destroyed, rendering the subject's records permanently unlinkable, while the **records themselves are never deleted** and the **hash chain and Merkle anchors stay intact and verifiable**.

This design is chosen precisely because it is **compatible with write-once-read-many (WORM) retention**. A conventional delete would break the append-only chain (producing a linkage-break or node-missing integrity failure) and would be impossible on genuinely write-once media. Crypto-shred satisfies erasure by removing the *ability to re-identify* rather than the *record*, so the integrity guarantee and any WORM retention obligation are preserved simultaneously.

The erasure workflow provides:

- **DPO gating and dual-control.** Erasure is committed by a distinct DPO; a generic Approver cannot.
- **A cancellable hold window** (default ~5 minutes) before execution, cancellable via the erasure-cancel endpoint; a ticker executes due shreds.
- **A signed erasure certificate** sealed on commit, evidencing that the erasure occurred.

See the [erasure (crypto-shred) runbook](../runbooks/erasure-crypto-shred.md) for the operational procedure.

> **Warning:** Crypto-shred is **irreversible** and its scope is exact: it destroys the linkage key for the identified subject. Once the hold window elapses and the shred executes, the subject's records cannot be re-identified by any party, including the controller. Confirm the subject and the hold-window state before commit.

### 7.3 Physical retirement and external anchoring are prod-deltas

Two lifecycle steps a production retention program would require are **not** implemented in the reference and must not be represented as shipped:

- **Physical WORM retirement at COLD expiry** — the physical destruction or retirement of records once the COLD tier expires — is a prod-delta.
- **External WORM/object-lock anchoring and RFC 3161 trusted timestamps** — anchoring the chain to an external immutable store or a trusted timestamp authority — are prod-deltas. They are recommended production anchors that would further constrain the in-window residual described in Section 5, but the reference does not ship them.

The DPIA should record these as planned production controls with a defined owner, not as existing safeguards.

---

## 8. Standards and legal mapping

The cryptographic and regulatory anchors this design actually uses are enumerated in the [standards mapping](../reference/standards-mapping.md). In summary, the reference uses Ed25519 signatures (RFC 8032), a Merkle Signed Tree Head pattern modeled on Certificate Transparency (RFC 6962), scrypt key derivation (RFC 7914), and AES-256-GCM. GDPR articles engaged include Art. 4(5) and Recital 26 (pseudonymization), Art. 5 (principles and accountability), Art. 17 (erasure), Art. 25 (data protection by design and by default), Art. 30 (records of processing), Art. 32 (security of processing), and Art. 35 (DPIA). Where the operator processes personal data of data subjects in Korea, PIPA destruction and record-keeping obligations apply.

RFC 3161 trusted timestamps and S3 Object-Lock/WORM external anchoring are **prod-deltas** — present them as recommended production anchors, not as controls the reference provides.

---

## 9. DPIA risk register (summary)

| # | Risk | Mitigation in the reference | Residual / operator action |
| --- | --- | --- | --- |
| R1 | Human users wrongly challenged (false positive) | Low-FP design; heuristic rules challenge rather than block; monitor mode to measure before enforcing | Document FP residual and challenge accessibility (challenge WCAG UI is prod-delta) |
| R2 | Audit log is personal data, not anonymous | Pseudonymization at rest (Art. 4(5)) | Treat log as personal data across its lifecycle |
| R3 | Low-entropy identifier (e.g. IPv4) re-derivable if subject key/keystore leaks | Sealed keystore (scrypt N=2^15, AES-256-GCM); crypto-shred destroys the key | Protect `HMN_UNSEAL`; back up out-of-band; loss ≈ mass crypto-shred |
| R4 | Unauthorized re-identification | Vault + dual-control (distinct second role); action meta-audited | Control vault access; review re-identification audit trail |
| R5 | In-window history rewrite before next checkpoint | Witness co-sign stops silent rewrites; checkpoint every 32 records; public verification | Accept documented in-window residual; consider external anchoring (prod-delta) |
| R6 | Privileged admin misuse | RBAC, separation of duties, dual-control, deny-by-default, server-derived actor, meta-audit | Deploy production admin auth (mTLS/SSO is prod-delta) |
| R7 | Over-retention | HOT/WARM/COLD classifier | Reconcile tiers with legal schedule; physical retirement is prod-delta |
| R8 | Erasure breaking integrity / WORM conflict | Crypto-shred (key destruction), records retained, chain intact | Confirm subject and hold window; erasure is DPO dual-control |

---

## Related reading

- [Data-processing inventory](../reference/data-processing-inventory.md) — field-level record of what is processed and stored.
- [Standards mapping](../reference/standards-mapping.md) — cryptographic and regulatory anchors used versus aspirational.
- [Erasure (crypto-shred) runbook](../runbooks/erasure-crypto-shred.md) — operational erasure procedure.
- [RBAC and separation of duties](../reference/rbac-separation-of-duties.md) — roles, capabilities, and dual-control model.
- [Production vs reference](../reference/production-vs-reference.md) — the full prod-delta boundary.
