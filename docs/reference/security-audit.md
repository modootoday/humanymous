# humanymous — security & code audit report

*Pre-release audit. Method: a multi-reviewer Red/Blue process (adversarial finders per
dimension, independent verifiers that tried to refute each finding, and a final
evaluation judge), anchored to the standards in §5. This report is published for
transparency; it is not a certification.*

- **Audit date:** 2026-07-22 · **Target:** `main`
- **Verdict:** the audit returned *GO with fixes*. **Every confirmed finding
  (High → Low) has since been remediated at the code level** in a pre-release
  hardening pass; only the accepted/residual risks in §4 remain. The full list of
  applied fixes is in the [CHANGELOG](../../CHANGELOG.md) *Security* / *Fixed* sections.

## 1. Scope & asset inventory

**Reviewed:** detection engine (`cmd/server`, `:8443`); Gate reverse-proxy edge
(`cmd/gate`, `:8444`) and admin plane (`:8445`); humanymous Pass (`internal/pass`,
`web/pass.html`); the reverse-proxy forwarding path; the WASM/JS loader; the
tamper-evident audit chain (`internal/audit`); resource watermarking; the crypto axes
(PoW, attestation); public docs. **Not reviewed:** third-party dependency internals
(covered by `govulncheck` + Trivy in CI); the production ACME/CDN edge; formal
cryptographic proofs.

## 2. Methodology & tooling

Static review guided by the **OWASP Code Review Guide** and **OWASP ASVS** (target
Level 2 for the web surface); dynamic/behavioural review mapped to **OWASP WSTG** via
the repo's harnesses — the 26-profile Docker attack catalog, the 34-check Gate
conformance suite, the multi-subnet correlation swarm, the 8-round Pass red/blue
wargame, and dedicated **reverse-proxy forwarding-fidelity** tests
(`internal/gate/forward_fidelity_test.go`). Supply-chain posture assessed against
**OpenSSF Scorecard** and **SLSA v1.0**.

## 3. Remediation summary

All confirmed findings were fixed and verified (`go test ./...` green; Pass e2e 5/5;
Pass wargame gate PASS; Docker engine attack 26/26 detected with 0 bypass; Gate
conformance 34/34; images build with third-party licences bundled). Categories fixed,
with the durable controls added:

- **Admin plane** — no unauthenticated operator-token handout; loopback-default bind;
  tokens never logged; fail-closed on placeholder/low-entropy secrets.
- **Honesty** — every "100% / 0% FPR" absolute replaced with bounded, reference-measured
  language across README + the results page (enforces the project's own style guide).
- **humanymous Pass accessibility** — no hidden timeout (honest, announced expiry);
  screen-reader-announced outcomes; a real non-drag pointer path (on-screen tap
  controls) and completed slider semantics; a support-contact escape route; readable
  hint contrast.
- **Anti-replay** — the interaction-trace digest is quantized, so sub-millisecond noise
  no longer defeats replay detection.
- **Reverse-proxy fidelity** — migrated to the modern `Rewrite` hook so
  `X-Forwarded-For` is a single authoritative socket-derived value (a duplicate was
  found and fixed); strict tests assert client headers/cookies/body/method reach the
  upstream intact, the upstream's status/Set-Cookie/headers return to the client, and
  forged trust headers are blocked before forwarding.
- **Privacy** — the watermark IP token is now a **keyed** HMAC pseudonym (not a
  reversible bare hash); the watermark ledger is in the data-processing inventory with
  its TTL/erasure scope; a GDPR Art. 13/14 collection-notice snippet is documented.
- **Supply chain** — `govulncheck`, CodeQL, and Trivy image scanning gate CI; Dependabot
  covers Go modules + Actions; build version is stamped into the binaries; licences ship
  in the images.
- **Cookies** now set `Secure`; key/id CSPRNG seeding fails closed; the in-memory Pass
  session map is bounded.

## 4. Accepted / residual risk (by design, disclosed — not defects)

These are inherent trade-offs, documented rather than "fixed":

- **Detection floor.** A *perfect* human-like forgery from a *fresh* identity can still
  clear the Pass puzzle — the fundamental limit of any accessible challenge. It is
  bounded by the attestation issuance rate and folded engine risk, and the engine
  **DENIES** the session the moment any independent bot signal (JA4/L1–L7/correlation)
  appears; solving Pass never launders a session already proven a bot.
- **False-positive posture.** The reference FPR is a DENY-only metric and does not
  measure real-human friction or disaggregate assistive-tech / uncommon-browser /
  privacy-hardened users. Disclosed in the [transparency report](../explanation/transparency-report.md).
- **Attestation axis is session-scoped by design.** To keep the normal/accessible lane
  token-free, the rate-limited attestation gate triggers on session cadence; a
  cookie-rotating flood from one fingerprint is instead caught by the **fingerprint-keyed
  velocity engine-fusion** (`l7.pass.flood` → risk → non-ALLOW verdict), a different but
  effective axis.
- **Anti-injection CSP is report-only by design** — violation telemetry for a detection
  engine, not an enforced block (stated plainly in the docs).
- **Admin-plane security** rests on loopback binding + no auto-issued token; a production
  operator should front it with **mTLS/SSO**.
- **Audit tamper-evidence** resists external tampering and enables post-hoc verification,
  but not an attacker with in-process/code control — production runs the witness and
  keys out-of-process/HSM.
- **Supply-chain hygiene, ongoing:** GitHub Actions SHA-pinning and base-image digest
  pinning are managed by Dependabot rather than hand-pinned.

## 5. Standards referenced

OWASP ASVS · OWASP Code Review Guide · OWASP WSTG · CWE · ISO/IEC 29147 & 30111 (CVD) ·
OpenSSF Scorecard & Best Practices Badge · SLSA v1.0 · SPDX/CycloneDX (SBOM) ·
NIST SSDF · W3C "Inaccessibility of CAPTCHA" & WCAG 2.2 · GDPR (Arts. 5/6/13/17/22).

*Report vulnerabilities via [`SECURITY.md`](../../SECURITY.md) /
[`security.txt`](https://humanymous.net/.well-known/security.txt).*
