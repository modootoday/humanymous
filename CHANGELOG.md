# Changelog

All notable changes to humanymous are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Security-relevant changes
are called out in a dedicated **Security** subsection with upgrade urgency.

## [Unreleased]

Pre-release hardening pass driven by a multi-reviewer Red/Blue + judge code audit
(see [`docs/reference/security-audit.md`](docs/reference/security-audit.md)). Release
readiness moved from *GO-with-fixes* toward *GO* by clearing the confirmed blockers.

### Security

- **Admin plane no longer hands out a bearer token to unauthenticated clients.** The
  Gate admin console injects a live operator token **only in explicit local-demo mode**
  (`HMN_ALLOW_DEV_TOKENS=1`); in a real deployment the console loads without a token and
  admin APIs require authentication. The admin listener now **defaults to loopback**
  (`-admin-addr 127.0.0.1:8445`), and a startup warning fires if it is bound off-host
  without a mutually-authenticated front. *(audit SEC-1, CWE-306/CWE-522 — upgrade
  strongly recommended for any off-host admin exposure.)*
- **Admin bearer tokens are no longer printed to logs.** Boot logging emits token
  values only in local-demo mode; otherwise it prints role names only, so env-supplied
  production tokens never reach stdout / container logs. *(audit SEC-2, CWE-532.)*
- **Fail-closed on placeholder / weak admin secrets.** The boot guard now refuses to
  start on `CHANGE-ME`/placeholder admin tokens or a weak `HMN_UNSEAL` passphrase
  (min-entropy check), so `cp .env.example .env && up` cannot go live on shipped
  defaults. *(audit SEC-3, CWE-1188/CWE-798.)*
- **Pass anti-replay strengthened.** The interaction-trace digest is quantized before
  hashing, so a replayed human trace still collides after sub-millisecond noise is added
  to defeat exact matching. *(audit, CWE-294.)*
- **Session/verdict cookies now set `Secure`.** *(audit, defense-in-depth over
  HTTPS-only deployment.)*
- **Supply chain:** CI now runs `govulncheck` and a Dependabot config was added
  (Go modules + GitHub Actions). *(audit — OpenSSF Scorecard baseline.)*
- **License compliance:** `LICENSE`, `NOTICE`, and a new `THIRD_PARTY_LICENSES.md`
  index now ship inside the release container images. *(audit — BSD-3/MIT redistribution.)*

### Added

- `SECURITY.md` — coordinated vulnerability disclosure policy (ISO/IEC 29147/30111
  aligned) with SLAs and safe harbor.
- `/.well-known/security.txt` (RFC 9116) on the documentation site.
- `docs/reference/security-audit.md` — the full code-audit report (findings, severities,
  remediation status, residual risk).
- `docs/explanation/transparency-report.md` — public transparency report (how it decides,
  data handling, accessibility, dual-use posture, honest limitations, appeal path).
- `THIRD_PARTY_LICENSES.md` — third-party dependency licence index.
- humanymous Pass: a documented accessibility **escape route** (support contact + help
  link) reachable from the challenge itself.

### Changed

- **Honest metrics.** Every "100% / 0% false-positive / FPR 0%" absolute in `README.md`
  and `docs/report.html` was rewritten to bounded, reference-measured language that
  states the run is `n=1` on maintainers' hardware, that the "human" baseline is a
  Playwright/CDP session (not a physical human), and that the DENY-only FPR under-reports
  human friction. *(audit HON-1 — enforces the project's own style guide.)*
- The README anti-injection row now states the CSP ships **report-only** (violation
  telemetry), not an enforced block.

### Fixed

- **humanymous Pass accessibility (WCAG):** the hidden ~60–90s challenge timeout that
  contradicted the "No timer — take your time" banner is replaced by a ~10-minute TTL,
  and an expiry now returns an honest, screen-reader-announced message instead of a
  false "keys not in the slot" (WCAG 2.2.1); submit outcomes are announced to screen
  readers via an `aria-live` status region (WCAG 4.1.3). *(audit ACC-1, ACC-2, ACC-3.)*

### Known / residual (tracked, not yet closed)

See the [security audit report](docs/reference/security-audit.md) for the full list:
weak IP pseudonymization label, the watermark ledger's inclusion in the data-processing
inventory, a collection-time GDPR notice, CodeQL + image scanning, minor Pass WCAG items
(single-pointer drag alternative, slider role), the attestation gate's fingerprint-level
trigger, and pinning GitHub Actions to SHAs. The honest detection floor (a perfect
human-like forgery from a fresh identity can still clear the Pass puzzle) is an accepted,
documented limitation, not a bug.
