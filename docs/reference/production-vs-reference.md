---
title: Production-ready vs reference (prod-delta) & local↔production checklist
---

# Production-ready vs reference (prod-delta) & local↔production checklist

**Diátaxis quadrant:** Reference. **Audience:** engineering leaders, security reviewers, and integrators promoting humanymous Sentinel from a local reference build toward a production deployment.

This is the honesty page. This repository is a **reference implementation, not a production-hardened build.** humanymous Sentinel ("Sentinel" after first mention) is the reverse-proxy enforcement layer: it terminates TLS, streams the detection bundle into HTML, scores layers L1–L7 inline, enforces the verdict at the edge, and writes every decision to a tamper-evident audit log in front of an origin app it does not control.

The reference build demonstrates every mechanism end-to-end so you can evaluate it, but several components ship as dev-grade stubs that are safe on a laptop and unsafe in production. This page names each one — what the reference ships versus what a production deployment must supply (the "prod-delta") — and then gives a checklist for the local→production promotion.

For a component-level inventory of flags and presets, see [CLI, config & policy reference](./cli-config-policy.md). For key handling specifics, see the [Key management guide](../how-to/key-management.md). For install prerequisites, see [Install requirements](./install-requirements.md). For version-to-version moves, see the [Upgrade & migration guide](../how-to/upgrade-migration.md).

> **Warning:** Nothing that is dev-only in the reference build should reach production. That specifically means ephemeral in-memory keys, the self-signed in-memory TLS certificate, the printed bearer dev tokens, and the in-process (single-node) verdict and ban stores. Each has a production replacement listed below; shipping the dev-grade version is a security and availability risk, not a shortcut.

---

## Component-by-component: reference vs prod-delta

Each row states what the reference binary ships and what a production deployment is responsible for adding or replacing. "prod-delta" means the item is deliberately out of scope for the reference and is a production responsibility.

| Component | What the reference ships | Production responsibility (prod-delta) |
|-----------|--------------------------|----------------------------------------|
| **TLS certificate** | A self-signed certificate minted in memory at boot. Fine for `localhost`, rejected by real clients. | Real certificates via ACME or bring-your-own, backed by a real KMS/HSM for key custody. |
| **Node signing / HMAC / vault keys** | Ephemeral by default: a restart mints a **new** Ed25519 signing key (verifier public key changes) and a **new** vault (all pseudonym linkage lost ≈ accidental mass crypto-shred). A sealed keystore (`-keystore` + `HMN_UNSEAL`) makes them persistent. | Always run with `-keystore` + `HMN_UNSEAL` for a stable identity; back up the passphrase out-of-band. See [Key management](../how-to/key-management.md). |
| **Automated key rotation** | Not implemented for the signing/HMAC keys. (The verdict-token epoch key rotates every 15 min in-process; that is separate.) Rotation would require re-anchoring the chain. | An operational rotation procedure with re-anchoring. prod-delta. |
| **Admin authentication** | Bearer dev tokens over the separate admin listener, constant-time compared. Tokens are random per boot (printed at startup) unless set via `HMN_ADMIN_TOKENS`. | mTLS and/or SSO for the admin plane. prod-delta. |
| **Verdict store & bans (fleet state)** | In-process, single node. Verdict trust tokens and IP/fingerprint bans live in memory on one Sentinel instance. | A shared store (for example Redis) so verdicts and bans are consistent across a fleet. prod-delta. |
| **TLS fingerprint capture (JA4)** | Reference-level capture. Raw ClientHello TLS-accept-loop capture is not shipped. | Raw ClientHello TLS-accept-loop capture for full JA4 fidelity. prod-delta. |
| **Audit-log verification** | Verification logic lives in `internal/audit` (`Verify` / `SelfVerify`) and runs **live** in the Audit Console "Integrity" view and `GET /__hmn/admin/integrity` — verifies with the public key alone. | A standalone offline verifier **process/binary** (built from `internal/audit.Verify` over exported checkpoints). Not shipped in `cmd/`. prod-delta. See [Verify the audit log](../how-to/verify-audit-log.md). |
| **Retention retirement** | Retention tiers exist (HOT ~90d / WARM ~1y / COLD ~7y). | Physical retirement of retention segments to WORM storage. prod-delta. |
| **Live console updates** | The Audit Console refreshes/polls; a manual refresh button re-verifies the chain on demand. | SSE live-push for real-time console updates. prod-delta. |
| **False-positive triage UI** | No dedicated FP/appeal-queue view. Triage is done from the Overview feed plus Sessions drill-down. | A dedicated FP/appeal-queue view. prod-delta. |
| **Challenge / PoW interstitial** | A minimal accessible interstitial (HTTP 401, `no-store`, `lang="en"`, a plain-language message, loads the control-plane PoW loader). The code states production self-hosts the WCAG UI. | A full self-hosted WCAG 2.2 AA-conformant challenge experience. prod-delta. See [Challenge accessibility](../help/challenge-accessibility.md). |
| **Observability export** | The audit stream (`GET /audit` with filters + cursor), the Integrity view/endpoint, and the Overview KPIs. There is **no** Prometheus `/metrics` endpoint and **no** health/readiness probe in the reference. | A metrics endpoint, health/readiness probes, and SIEM log shipping. prod-delta. See [Observability & SIEM](../how-to/observability-siem.md). |

> **Note:** The `/health` route is an origin app path mapped to the `off` preset (a bypass), not a Sentinel health probe. Do not treat it as a readiness signal for Sentinel itself.

---

## Local → production checklist

Work through this before any deployment that faces real traffic. Each item removes one dev-only behavior described above.

### Identity and keys

- [ ] **Run with a sealed keystore.** Set `-keystore <path>` and `HMN_UNSEAL` so the signing key, per-record HMAC key, and pseudonym vault persist across restarts. Without this, keys are ephemeral and a restart is an accidental mass crypto-shred (cryptographic erasure) — the verifier public key changes and all pseudonym linkage is lost.
- [ ] **Back up `HMN_UNSEAL` out-of-band.** Losing the passphrase means the sealed identity cannot be opened (you lose chain-signing continuity and vault linkage ≈ mass crypto-shred). Store it separately from the keystore file. See [Key management](../how-to/key-management.md).
- [ ] **Plan key rotation as an operational procedure.** Automated rotation of the signing/HMAC keys is not implemented and requires re-anchoring; decide who rotates, when, and how the chain is re-anchored.

### Transport and origin trust

- [ ] **Replace the self-signed dev certificate** with real certificates (ACME or bring-your-own) backed by a real KMS/HSM. The in-memory self-signed cert is for `localhost` only.
- [ ] **Set a real `-origin-key`.** Provide a stable origin-cloaking HMAC key so the origin can validate `X-Hmny-Origin-Auth`. If left unset the key is random and ephemeral, and the origin cannot reliably distinguish Sentinel-forwarded traffic from a direct hit.

### State and fleet

- [ ] **Externalize fleet state.** The reference keeps verdict trust tokens and IP/fingerprint bans in-process on a single node. For more than one Sentinel instance, move them to a shared store (for example Redis) so verdicts and bans are consistent fleet-wide.

### Admin plane

- [ ] **Replace bearer dev tokens with real admin auth (mTLS and/or SSO).** Do not carry the printed dev tokens (or a static `HMN_ADMIN_TOKENS` value) into production. Preserve the RBAC role separation — Auditor, Operator, Approver, DPO — and keep dual-control intact (a distinct Approver commits a permanent/CIDR ban or the kill switch; a distinct DPO commits an erasure). See [RBAC & separation of duties](./rbac-separation-of-duties.md).

### Verification, observability, and challenge UX

- [ ] **Provide an offline audit verifier** if your compliance posture requires independent verification off-box. Build it from `internal/audit.Verify` over exported checkpoints; it is not shipped in `cmd/`. The live console "Integrity" view still verifies with the public key alone in the meantime.
- [ ] **Add observability integration.** Wire the audit stream into your SIEM and add the metrics/health probes your platform expects; none ship in the reference. See [Observability & SIEM](../how-to/observability-siem.md).
- [ ] **Self-host the full WCAG challenge UI.** The reference serves only a minimal accessible interstitial. Any accessibility conformance statement applies to the deployed challenge you host, not to the reference page. See [Challenge accessibility](../help/challenge-accessibility.md).
- [ ] **Plan for retention retirement.** The reference defines retention tiers but does not physically retire segments to WORM; add that if your retention policy requires it.

> **Warning:** Cryptographic erasure (crypto-shred) is irreversible — it destroys the per-subject linkage key while the hash chain and Merkle anchors stay intact. Running with ephemeral keys turns every restart into an unintended fleet-wide erasure of pseudonym linkage. Confirm the keystore checklist items above before production.

---

## What is *not* a prod-delta

Some limitations are design boundaries, not stubs a production build removes:

- **The T4 ceiling.** Anti-detect tooling combined with real-human click-farms (tier T4) is an explicit design boundary, not something a production deployment "fixes." It is mitigated only by rate and reputation controls, never eliminated.
- **The unanchored in-window residual.** The audit log is **tamper-evident**, not tamper-proof: records after the last signed checkpoint remain re-writable by the writer until the next checkpoint (every 32 records). This is an honestly-scoped property, not a gap a production build closes.
- **Fail-open on safe-method GET/HEAD (non-strict routes).** This is a documented accepted residual covered by fingerprint/subnet rate metering, chosen deliberately — not a stub.

For the reasoning behind these boundaries, see [What Sentinel is](../explanation/what-sentinel-is.md) and [Hard rules & verdicts](./hard-rules-verdicts.md).

---

## Related pages

- [Key management](../how-to/key-management.md) — sealed keystore, `HMN_UNSEAL`, rotation as an operational concern.
- [Install requirements](./install-requirements.md) — prerequisites and build.
- [Upgrade & migration](../how-to/upgrade-migration.md) — moving between versions.
- [CLI, config & policy reference](./cli-config-policy.md) — every flag, preset, and default.
- [RBAC & separation of duties](./rbac-separation-of-duties.md) — admin roles and dual-control.
- [Verify the audit log](../how-to/verify-audit-log.md) — live and offline verification.
- [Observability & SIEM](../how-to/observability-siem.md) — audit stream, metrics, log shipping.
