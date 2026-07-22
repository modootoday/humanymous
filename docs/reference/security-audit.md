# humanymous — security & code audit report

*Pre-release audit. Method: a multi-reviewer Red/Blue process (adversarial finders per
dimension, independent verifiers that tried to refute each finding, and a final
evaluation judge), anchored to the standards below. This report is published in the
spirit of transparency; it is not a certification.*

- **Audit date:** 2026-07-22 · **Target:** `main`
- **Verdict:** **GO with fixes** — the engine is functionally strong; every confirmed
  blocker was small and has since been remediated (see status column). Residual items
  are tracked below.

## 1. Scope & asset inventory

**Reviewed:** detection engine (`cmd/server`, `:8443`); Gate reverse-proxy edge
(`cmd/gate`, `:8444`) and admin plane (`:8445`); humanymous Pass challenge
(`internal/pass`, `web/pass.html`); the WASM/JS loader; the tamper-evident audit chain
(`internal/audit`); resource watermarking; the crypto axes (PoW, attestation); public
docs (`README.md`, `docs/`). **Not reviewed:** third-party dependency internals (covered
only by `govulncheck`); the production ACME/CDN edge; formal cryptographic proofs.

## 2. Methodology & tooling

Static review guided by the **OWASP Code Review Guide** and **OWASP ASVS** (target
Level 2 for the web surface); dynamic/behavioural review mapped to **OWASP WSTG** via
the repo's own harnesses — the 26-profile Docker attack catalog, the 34-check Gate
conformance suite (`gate-e2e`), the multi-subnet correlation swarm, and the 8-round
Pass red/blue wargame. Supply-chain posture assessed against **OpenSSF Scorecard** and
**SLSA v1.0**. Findings carry a severity and, where applicable, a **CWE**.

## 3. Findings

Severity reflects impact on the shipped reference deployment. **Status** is as of this
report.

### High

| ID | Finding | CWE | Status |
|----|---------|-----|--------|
| SEC-1 | **Admin-plane auth bypass** — the admin console served a live operator bearer token (`window.__HMN_TOKEN`) to any TLS client reaching `:8445`, which defaulted to all interfaces with no mTLS. | 306, 522 | **Fixed** — token injection gated to local-demo mode; admin listener defaults to loopback; off-host-without-mTLS startup warning. |
| SEC-2 | **Admin bearer tokens printed to logs** on every boot, including env-supplied production tokens (captured by `docker logs` / shippers). | 532 | **Fixed** — values logged only in local-demo mode; otherwise role names only. |
| HON-1 | **Flagship overclaims** — `README.md` and `docs/report.html` shipped `100%` / `zero false positives` / `FPR 0%` absolutes (contradicting the project's own style guide); the 0% FPR rested on an `n=1` Playwright/CDP pseudo-human. | — | **Fixed** — rewritten to bounded, reference-measured language; report captions added; baseline disclosed as Playwright/CDP, FPR defined as DENY-only. |
| ACC-1 | **Hidden ~60–90s Pass timeout** under a "No timer — take your time" banner; on expiry the server returned a false "keys not in the slot". | — (WCAG 2.2.1) | **Fixed** — TTL raised to ~10 min; expiry returns an honest, announced "this check expired — press New puzzle". |
| ACC-2 | **Pass submit outcomes not announced** to screen readers (written only to a non-live element). | — (WCAG 4.1.3) | **Fixed** — status region is now `role="status" aria-live="polite"`. |
| ACC-3 | **No alternative modality / in-challenge escape** for users who cannot solve the puzzle. | — (W3C CAPTCHA; GDPR Art. 22) | **Fixed** — the challenge now links to accessibility help and a support-contact "let me through another way" escape. |

### Medium

| ID | Finding | CWE | Status |
|----|---------|-----|--------|
| SEC-3 | `CHANGE-ME` default admin tokens and a placeholder `HMN_UNSEAL` booted successfully. | 1188, 798 | **Fixed** — fail-closed on placeholder/low-entropy secrets outside demo mode. |
| PASS-1 | Pass anti-replay bypassable via sub-ms trace perturbation / post-window exact replay; a code comment overstated its strength. | 294 | **Fixed** — trace quantized (1 ms / 0.05 pressure) before hashing; documented as best-effort. |
| CSP-1 | Anti-injection CSP is **Report-Only** on core and Gate; README listed it as a live "guard". | 693 | **Partly fixed** — README corrected to "report-only telemetry". Enforcing CSP with a nonce is a tracked enhancement. |
| SUP-1 | CI lacked `govulncheck` / CodeQL / image scan / Dependabot. | — | **Partly fixed** — `govulncheck` + Dependabot added; CodeQL + image scan tracked. |
| LIC-1 | Third-party BSD-3/MIT texts not shipped with images/binaries. | — | **Fixed** — `LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.md` now copied into images. |
| PRIV-1 | Weak pseudonymization — an **unkeyed** truncated SHA-256 of a raw IP labelled a "privacy" control. | 916 | **Open** — route through the keyed vault or drop the label (tracked). |
| PRIV-2 | The resource-watermark ledger is a second personal-data store outside the data-processing inventory and erasure boundary. | — | **Open** — add to the inventory + document TTL/erasure scope (tracked; disclosed in the transparency report). |
| PRIV-3 | No collection-time GDPR Art. 13/14 transparency notice mapped in the operator docs. | — | **Open** — notice snippet to be added to the operator/DPO docs. |
| ATT-1 | The attestation identity-gate triggers on **session** velocity, so a cookie-rotating flood does not hit the rate-limited identity budget. | — | **Open / mitigated** — a cookie-rotating single-fingerprint flood is still flagged by the **fingerprint-keyed velocity engine-fusion** (it accrues `l7.pass.flood` → risk → verdict). Extending the attestation trigger to the fingerprint level is tracked. |
| ACC-4 | Pass lacks a single-pointer drag alternative (WCAG 2.5.7); the `role="slider"` rows are half-implemented (WCAG 4.1.2). | — | **Open** — the keyboard lane already provides a non-drag path; on-screen per-row controls + slider ARIA are tracked. |

### Low / informational

| ID | Finding | Status |
|----|---------|--------|
| LOW-1 | Cookies missing `Secure`. | **Fixed.** |
| LOW-2 | `crypto/rand` errors ignored when seeding some Gate keys (bounded on Go 1.24+, which crashes rather than returning a short read). | **Open** — align with the fail-closed `randHex` pattern (tracked). |
| LOW-3 | Unbounded growth of the in-memory Pass trace/session maps. | **Open** — add an LRU/cap (tracked). |
| LOW-4 | GitHub Actions not pinned to SHAs; no embedded build/version metadata; base images on floating tags. | **Open** — pin actions (Dependabot now assists), stamp version, digest-pin bases (tracked). |
| INFO-1 | The audit chain's tamper-evidence resists external tampering and post-hoc verification, but a witness/keys held in-process do not resist an attacker with process/code control (documented in code; production runs the witness/keys out-of-process/HSM). | Accepted, disclosed. |
| INFO-2 | Strong positive baseline to preserve: SLSA provenance (`mode=max`), SPDX SBOM, cosign keyless signing by digest, distroless-nonroot static builds, dev-gated playground, loopback admin in the release compose, fail-closed `randHex` on the core. | — |

## 4. Objective-metrics baseline (honest)

- **Reference detection run (`n=1`, maintainers' hardware):** all 25 bot profiles in the
  local catalog were blocked (DENY/CHALLENGE) and the 1 baseline was not denied; Gate
  conformance 34/34; correlation swarm 15/15 DENY. **These are reference-measured, not a
  guarantee**, and the baseline is a Playwright/CDP session, not a physical human.
- **Supply chain / SLSA:** GitHub Actions release emits **signed provenance
  (`mode=max`)** + **SPDX SBOM** and **cosign**-signs the image by digest → **SLSA build
  track ~L1→L2**. We do **not** claim L3 or any "certified/compliant" badge.
- **OpenSSF Scorecard:** several checks now pass (Security-Policy, Signed-Releases,
  CI-Tests, Dependency-Update-Tool, Vulnerabilities via govulncheck); CodeQL (SAST) and
  branch-protection/action-pinning are tracked. Run Scorecard for current numbers rather
  than trusting this prose.

## 5. Residual & accepted risk

- **Detection floor (accepted):** a *perfect* human-like forgery from a *fresh*
  identity can still clear the Pass puzzle. This is the fundamental limit of any
  accessible challenge; it is bounded by attestation issuance rate + folded engine risk,
  and the engine still DENIES the session the moment any independent bot signal
  (JA4/L1–L7/correlation) appears. Solving Pass **never** launders a session already
  proven a bot (verified live in the wargame).
- **False-positive risk (accepted, disclosed):** the reference FPR is DENY-only and does
  not measure real-human friction or disaggregate assistive-tech / uncommon-browser /
  privacy-hardened users. See the transparency report.
- **Admin plane:** with the fixes above, admin security in the reference build rests on
  **loopback binding + no auto-issued token**; production should still front it with
  **mTLS/SSO**.
- **Dual-use:** the same fingerprinting/behavioural signals could be repurposed for
  surveillance; the project is defensive and local-target-only by design and policy.

## 6. Standards referenced

OWASP ASVS · OWASP Code Review Guide · OWASP WSTG · CWE · ISO/IEC 29147 & 30111 (CVD) ·
OpenSSF Scorecard & Best Practices Badge · SLSA v1.0 · SPDX/CycloneDX (SBOM) ·
NIST SSDF · W3C "Inaccessibility of CAPTCHA" & WCAG 2.2 · GDPR (Arts. 5/6/13/17/22).

*Report vulnerabilities via [`SECURITY.md`](../../SECURITY.md) /
[`security.txt`](https://humanymous.net/.well-known/security.txt).*
