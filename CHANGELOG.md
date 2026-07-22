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
- **Reverse-proxy forwarding fidelity:** migrated the Gate proxy to the modern
  `Rewrite` hook so `X-Forwarded-For` is a single authoritative socket-derived value
  (a duplicated value was found and fixed). Added strict tests asserting client
  headers/cookies/body/method reach the upstream intact, the upstream's status /
  Set-Cookie / headers return to the client, and forged trust headers
  (`X-Forwarded-For`, `X-Real-Ip`, `X-Hmny-Origin-Auth`, `Forwarded`, `Cf-Connecting-Ip`)
  are blocked before forwarding.
- **Keyed IP pseudonym:** the watermark ledger's IP token is now an HMAC pseudonym, not
  a reversible bare SHA-256. *(audit PRIV-1, CWE-916.)*
- **CSPRNG seeding fails closed** when generating Gate keys/session ids. *(audit LOW-2.)*
- **Bounded** the in-memory Pass session map. *(audit LOW-3.)*
- **Supply chain (more):** added CodeQL (SAST) and Trivy image scanning to CI; binaries
  are stamped with a build `version`. *(audit SUP-1 / LOW-4.)*

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
- **Constraint-resolution features (all OFF by default, behind flags; experimental —
  see the per-flag trust caveats).** Driven by a researched design blueprint:
  - **Shared fleet state via Redis** (`-redis host:port`): bans + sticky verdicts + a
    shared sliding-window rate limiter propagate across a Gate fleet, so a ban/DENY on
    one node is enforced on all. A Redis outage degrades each node to its local view
    (circuit-breaker fast-fail, no lockout). *Treat Redis as a trusted, network-isolated
    component (no AUTH/TLS/value-signing yet).*
  - **PROXY-protocol-v2 real-IP recovery** (`-trusted-proxies <cidrs>`): the Gate can
    sit behind an L4/TCP-passthrough balancer while keeping IP-keyed bans/rate/correlation
    correct; the PROXY header is honored ONLY from the trusted-CIDR balancers.
  - **Trust-upgrade signals** for legitimate automation / returning users: Web Bot Auth
    (RFC 9421, `-agent-keys`), Privacy Pass Private Access Tokens (RFC 9578,
    `-pat-issuers`), and WebAuthn possession assertions (`-webauthn-creds`). Each verifies
    a signature/token and forwards; a missing/invalid one is a no-op (never a deny).
  - **RFC 6962 Merkle audit tree**: inclusion + consistency proofs over the tamper-evident
    log (Merkle root folded into the signed tree head; witness co-signs only append-only
    extensions → split-view protection); read-only `/__hmn/admin/proof?seq=N`; durable
    ClickHouse projection (`-audit-clickhouse`).
  - **Streaming MAD anomaly SHADOW observer** (`-anomaly-shadow`): log-only, never affects
    the verdict — evidence collection before any signal earns weight.

### Changed

- **Maintainability refactor (behavior-preserving, detection FROZEN).** 19 refactors that
  make silent stringly-typed failures loud (signal-id resolution tests, launcher/catalog
  parity), split large files by concern, replace the admin dispatch switch with a
  declarative route+RBAC table, and add opt-in structured logging + audit
  event-id/correlation/latency + an upstream-error audit record.
- **Deployment-review ship-blockers cleared:** the Gate now runs a 1-minute GC ticker so
  its in-memory detection maps cannot grow without bound under fingerprint/IP churn
  (OOM-DoS fix, parity with the core engine), and the unused **JA4H** HTTP fingerprint
  (FoxIO License 1.1, not the BSD-3 that covers JA4-TLS) was removed as a dead-code
  licence liability.

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
- **humanymous Pass accessibility (more):** on-screen tap controls give pointer-only
  users a non-drag path (WCAG 2.5.7); each row is a fully-described, focusable slider
  with `aria-valuenow/min/max` (WCAG 4.1.2); hint-text contrast raised (WCAG 1.4.3).
  *(audit ACC-4.)*
- **Privacy docs:** the resource-watermark ledger is now in the data-processing
  inventory with its TTL/erasure scope, a GDPR Art. 13/14 collection-notice snippet was
  added, and the biometric-scope note resolved to a definitive design position.
  *(audit PRIV-2, PRIV-3.)*

### Accepted / residual (by design — see the audit report §4)

The remaining items are documented trade-offs, not open defects: the honest detection
floor (a perfect fresh-identity forgery can still clear the puzzle), the DENY-only FPR
posture, the session-scoped attestation axis (cookie-rotation is caught by the
fingerprint-velocity engine-fusion instead), the report-only anti-injection CSP, the
loopback-plus-mTLS admin posture, the in-process audit tamper-evidence scope, and
Dependabot-managed Action/base-image pinning.
