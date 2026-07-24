---
title: Glossary
description: "Definitions of the humanymous terms — verdicts, the seven layers, hard rules, fingerprints, and the Gate/Core/Ledger pieces."
---

# Glossary

Plain definitions of the terms a newcomer meets across the humanymous documentation. For how these fit together on a single request, see [How Gate sees a request](../concepts/how-gate-sees-a-request.md); for the exact rule and signal tables, see [Hard rules & verdicts](hard-rules-verdicts.md).

humanymous Gate is a reference implementation, not a production-hardened build. Definitions below describe the reference behaviour.

## Verdicts and scoring

**ALLOW.** The verdict for a request not scored as automated (risk 0–29), or one carrying a valid [verdict trust token](#verdict-trust-token) that took the fast path. At the edge it maps to the action `pass`: the request is forwarded to your [origin](#origin--upstream). On an [attested route](#attestation-floor--attested-route) an ALLOW is priced rather than fast-pathed.

**CHALLENGE.** The verdict for an ambiguous request (risk 30–69). At the edge it maps to `challenge_pow`: Gate serves an accessible [proof-of-work](#proof-of-work--pow-challenge) interstitial and does not contact the origin until the work is done. A score-based CHALLENGE — not one a hard rule promoted — can upgrade to ALLOW once the session solves the work.

**DENY.** The verdict for a request scored as automated (risk 70–100) or promoted by a high-confidence [hard rule](#hard-rule--hr-nn). At the edge it maps to `block`: the request is stopped and the origin is never contacted. Blocked visitors see plain language and an [incident handle](#incident-handle) only — never internal identifiers.

**Risk score.** A number from 0 to 100 (one decimal place) that the seventh layer aggregates from the signals the earlier layers observed; higher means more evidence of automation. Policy thresholds map the score to a verdict: 0–29 ALLOW, 30–69 CHALLENGE, 70–100 DENY. The score is deterministic — the same input produces the same score — and a hard rule can override where the score alone would have landed.

**Verdict.** The engine's decision for a request: one of ALLOW, CHALLENGE, or DENY, written uppercase. The edge also carries a fourth state, `none` (Unknown), for a request with no evidence captured yet, which fails open or closed depending on the route and HTTP method.

## The seven layers (L1–L7)

**L1–L7.** The seven detection layers a request is scored across. L1–L4 run in the browser (JavaScript and WebAssembly); L5–L6 run on the Go server; L7 is the aggregation and decision step. No single layer produces a verdict on its own — inter-layer disagreement carries more weight than any one signal.

- **L1 Static client** — reads static automation tells, such as the `webdriver` flag and headless indicators.
- **L2 Fingerprint** — hashes canvas, WebGL, and audio output plus screen and hardware traits.
- **L3 Client integrity** — detects patched native functions, proxies, and runtime hooks.
- **L4 Behavioral** — observes mouse, keystroke, and scroll patterns and the `isTrusted` flag on events.
- **L5 Network / protocol** — reads the [JA3/JA4](#ja3--ja4-tls-fingerprint) and [HTTP/2 fingerprint](#http2-fingerprint) and header order from the connection.
- **L6 Consistency cross-check** — checks that the user agent, client hints, in-browser evidence, and TLS all agree with each other.
- **L7 Scoring / decision** — aggregates the L1–L6 contributions into the risk score, then applies hard rules.

## Hard rules and fingerprints

<a id="hard-rule--hr-nn"></a>
**Hard rule (HR-NN).** A high-confidence pattern that can promote or override the score-based verdict — written with a hyphen and no space, for example HR-14. Engine-plane rules (HR-1 through HR-21) are evaluated inside the score pipeline and are the ones the [Detection Observatory](#detection-observatory) shows; Gate-plane rules (HR-22 through HR-30) are evaluated at the proxy edge only. Some hard rules are high-confidence (a spike reads as automated traffic); others are heuristic and can catch some real users, so a spike there warrants investigation. See [Hard rules & verdicts](hard-rules-verdicts.md) for the full table.

<a id="ja3--ja4-tls-fingerprint"></a>
**JA3 / JA4 (TLS fingerprint).** Derived identifiers computed from the fields in a client's TLS ClientHello — cipher suites, extensions, and their order — which tend to reflect the underlying network stack rather than the claimed browser. When these resolve to a different engine than the user agent claims, that disagreement is a cross-check signal. Capturing them requires that Gate or the Core terminate raw TLS directly; a re-terminating CDN in front removes them.

**HTTP/2 fingerprint.** A derived identifier from a client's HTTP/2 behaviour — settings frames, header-table sizing, and stream priorities — read at the same layer as the TLS fingerprint. Like JA3/JA4, it is compared against the claimed user agent; a mismatch on both the TLS and HTTP/2 fingerprints together is a strong cross-check tell.

**Cross-session correlation.** Linking requests that share a device [fingerprint](#fingerprint) across many different subnets or sessions, which can reveal one client rotating through residential-proxy addresses. It is one of the signals that feeds the score and, in its high-confidence form, a hard rule. It does not by itself identify a person; it binds only to signals already collected.

**Fingerprint.** A derived identifier for a client, combining signals such as TLS and HTTP/2 traits and device characteristics. It is used to bind tokens, meter rate, and correlate sessions — not to identify a named individual.

## The pieces

**Gate.** The reverse-proxy enforcement layer that fronts your [origin](#origin--upstream). It terminates TLS, streams the detection bundle into served HTML, scores each request across L1–L7, enforces the verdict at the edge before the origin is contacted, and writes every decision to a tamper-evident audit log. First mention on a page is "humanymous Gate", then "Gate". See [Which piece am I using?](../explanation/which-piece-am-i-using.md).

**Core.** The standalone detection engine (`cmd/server`) that embeds the same L1–L7 scoring as Gate and returns a verdict without edge enforcement or an admin plane. It is the reference and testing surface, and — because it can terminate TLS on its own accept loop — it captures JA3/JA4 and the HTTP/2 fingerprint directly. Gate and the Core share one scoring brain, so a request scores the same way in both.

**Ledger.** The admin single-page console, served from a separate admin listener, for reading enforced-and-recorded verdicts, audit integrity, bans, policy, and erasure. It is where operators work with hard-rule IDs, risk scores, and signal names; end users never see it.

<a id="detection-observatory"></a>
**Detection Observatory.** A read-only, developer-facing page hosted on the Core that shows the engine's per-signal scoring in isolation — the score trace, the hard-rule evaluations, and live self-test runs — with nothing enforced or recorded. It is dev-gated and refuses to start on a non-loopback address. It shows how a verdict was reached; the Ledger shows what the edge did to live traffic.

**Origin / upstream.** The application Gate sits in front of and forwards allowed requests to. "Origin" is the primary term; "upstream" is the name of the `-upstream` flag alias.

## Challenges, tokens, and attestation

<a id="proof-of-work--pow-challenge"></a>
**Proof-of-work (PoW) challenge.** CPU work a challenged session performs, served as an accessible interstitial from the control plane; it is not a CAPTCHA. Solving it (signal `l7.pow.solved`) can upgrade a score-based CHALLENGE to ALLOW, because it demonstrates cost. It never overrides a hard rule — it proves CPU effort, not that a person is present.

**humanymous Pass.** An interactive, LLM-resistant challenge presented on a CHALLENGE verdict, designed so that solving it is hard to automate. On an [attested route](#attestation-floor--attested-route), a successful solve mints a short-lived step-up receipt that the Gate exchanges for the `hmn_su` proof. The same Pass is solved by an unattested human and an anonymous visitor alike, so there is no categorical human block. See [Why am I seeing this?](../help/why-am-i-seeing-this.md).

<a id="attestation-floor--attested-route"></a>
**Attestation floor / attested route.** A route an operator marks with the `attested` preset (`strict` plus the floor), typically a high-value mutating path such as checkout or a password change. On such a route an ALLOW is priced rather than fast-pathed: absent possession — an existing WebAuthn, Privacy Pass, or Web Bot Auth trust-upgrade — or an `hmn_su` step-up proof from a Pass solve, the ALLOW is demoted to CHALLENGE→Pass, never to DENY. The floor prices high-value passes into a per-session human solve; it does not detect or resolve a coherent spoof. See [Configure attested routes](../how-to/configure-attested-routes.md).

<a id="verdict-trust-token"></a>
**Verdict trust token.** A fingerprint-bound token (`hmn_vt`) that records a prior verdict so a valid ALLOW can take a fast path without re-scoring. It is bound to the client fingerprint: a token replayed from a different fingerprint, or forged, is rejected rather than honoured. It is session-scoped and short-lived, and carries no new identifier.

<a id="rit-request-integrity-token"></a>
**RIT (request-integrity token).** A rotating, short-lived token that binds a request to its session and body via an HMAC, so that header or request-body tampering is detectable. A token that fails the body HMAC, or a RIT that is replayed or absent on an API call, is treated as a tamper or heuristic signal. Like the verdict trust token, it is session-scoped, not persisted to the audit log, and adds no new personal-data category.

**Incident handle.** The opaque reference an end user is given on a challenge or block page so they can appeal. It is the only identifier shown to blocked visitors — it maps to a record operators can look up, without revealing any detection detail.

## Threat model

<a id="t0t4-threat-tiers"></a>
**T0–T4 threat tiers.** A five-tier scale used in threat-model and self-validation docs to describe the automation the design is meant to resist. It describes capability classes, not real users: **T0** non-browser HTTP clients; **T1** naive Selenium or Puppeteer; **T2** stealth-patched automation with residual leaks; **T3** real-engine automation where behaviour and network become the deciding — and lower-confidence — signals; **T4** anti-detect tooling combined with real-human click-farms.

**The T4 ceiling.** An explicit design boundary: T4 is not solved. Real humans clicking through anti-detect browsers cannot be separated from legitimate users by client or network signals alone, so the design does not claim to detect them by those signals — it mitigates the tier through rate limiting, reputation, and, on attested routes, by pricing the ALLOW. This documentation does not claim otherwise anywhere.

## Related

- [How Gate sees a request](../concepts/how-gate-sees-a-request.md) — the request lifecycle and the shared vocabulary in context.
- [Hard rules & verdicts](hard-rules-verdicts.md) — the full hard-rule and signal-ID lookup tables.
- [Which piece am I using?](../explanation/which-piece-am-i-using.md) — Gate, the Core, and the Observatory compared.
