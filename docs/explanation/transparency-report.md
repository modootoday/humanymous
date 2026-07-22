# humanymous — transparency report

*This report explains, in plain language, what humanymous does, how well it works, what
data it uses, and where its limits are. It is written to be honest rather than
flattering: humanymous **raises the cost of automated abuse; it does not claim to stop
all of it.***

Last updated: 2026-07-22.

## What humanymous is, and how it decides

humanymous is a **defensive bot-detection system**. When a website puts it in front of a
page, it scores each visiting session and returns one of three outcomes:

- **ALLOW** — the request proceeds normally.
- **CHALLENGE** — the session looks ambiguous, so the person is asked to clear an
  interactive check ("**humanymous Pass**") before proceeding.
- **DENY** — strong evidence of automation; the request is blocked at the edge.

**An automated system makes this determination.** Clearing the Pass challenge can upgrade
a *CHALLENGE* to *ALLOW*, but it **cannot** override a *DENY* that other signals already
justified — solving the puzzle never "launders" a session that is independently shown to
be a bot.

## How it works, at a high level

Without revealing enough to help evade it, the signals fall into layers: network/TLS
fingerprint (JA3/JA4, HTTP/2 profile), HTTP header consistency, browser/runtime
fingerprint, behavioural timing (pointer/keystroke **timings** — never the keys typed),
and cross-session correlation. The Pass challenge additionally checks a proof-of-work,
an optional rate-limited attestation token, and that the interaction came from a real
device — **not** that the puzzle is hard (it is deliberately solvable, including by
screen-reader users).

## How well it works (and how to read the numbers)

In a **single reference run (`n=1` per profile) on the maintainers' hardware** against a
26-profile local catalog (**25 bot profiles + 1 baseline**), all 25 bot profiles were
blocked and the baseline was not denied. **This is reference-measured, not a guarantee.**

Please read these caveats with any figure you see:

- The "baseline" is a **Playwright/CDP-driven session, not a physical human**, so a
  real-human false-positive rate is **not** measured here.
- Our "false-positive rate" is a **DENY-only** metric: it cannot count a real person who
  was sent to a *CHALLENGE*, so it **under-reports human friction**.
- We do not have measured error rates disaggregated across **assistive-technology users,
  uncommon or older browsers, or privacy-hardened / VPN / Tor** configurations. Those
  populations may see more challenges; we treat that as a known risk, not a solved one.

## If you were blocked or challenged (notice & appeal)

If humanymous blocks or challenges you, the page tells you **an automated system acted**
and gives a reason category. If an interactive challenge is not workable for you, you can
**reach a human**: email **support@modoo.today** (the Pass challenge links to this and to
[accessibility help](../help/challenge-accessibility.md)). We aim to provide a
**meaningful, non-automated review** of contested decisions — consistent with GDPR
Art. 22 and the Santa Clara Principles — not a rubber stamp.

## Data we use

- **Collected:** IP address, TLS fingerprint (JA3/JA4), HTTP/2 fingerprint, User-Agent
  and Client-Hints, SNI, a browser/device fingerprint, and behavioural **timings**
  (pointer/wheel/keystroke inter-event timing and, on mobile, device-motion presence).
  **Keystroke *values* are never recorded.** A per-session first-party **resource
  watermark** is used only for leak tracing.
- **Legal basis:** legitimate interest (GDPR Art. 6(1)(f), Recital 49 — network and
  information security), backed by a documented legitimate-interest assessment. Purpose
  is **limited to security**; the signals are **never** repurposed for analytics or
  advertising.
- **Sharing:** humanymous is **self-hosted** by the operator; there is no third-party
  data sharing built in.
- **Privacy signals:** Global Privacy Control / Do-Not-Track are read as inputs that
  *reduce* false positives; because processing is for security, they are not treated as
  an opt-out of the security determination itself.

> **Note for operators:** you remain the data controller. humanymous does not make you
> GDPR-compliant on its own — you must provide the collection-time notice, retention
> policy, and lawful basis for your deployment. See the operator docs.

## Retention & erasure

Telemetry, IP/fingerprint bans, and the audit log have bounded retention configured by
the operator. The admin plane supports **cryptographic-shred erasure** (deleting the
linkage key renders pseudonymized records unlinkable) with two-person control. One store
is out of that path and disclosed here for honesty: the in-memory **resource-watermark
ledger self-expires on a ~24-hour TTL** and is not part of the shred flow.

## Accessibility of the Pass challenge

Pass appears **only on a CHALLENGE verdict** (we minimise when a challenge is needed),
**never locks you out**, keeps cognitive load low, has **no time limit**, and provides a
**keyboard lane with screen-reader announcements**. Honestly, though — per the W3C
["Inaccessibility of CAPTCHA"](https://www.w3.org/TR/turingtest/) note — **no interactive
challenge is universally accessible.** That is why there is always a **crypto/no-puzzle
path** and a **support-contact escape** to be let through another way. Some Pass
accessibility refinements (an on-screen non-drag control, richer slider semantics) are
still in progress; see the [security audit](../reference/security-audit.md).

## Dual-use posture

humanymous is built and licensed for **defensive** use — detecting and challenging abuse
of your own service. It ships a red-team catalog (Selenium/Puppeteer/Playwright-class
profiles) whose **only** purpose is to validate *your own* detector; using it against
systems you do not own is out of scope and not authorised. We acknowledge that
fingerprinting and behavioural signals **could** be repurposed for surveillance; our
safeguards are data-minimisation, a security-only purpose, no ad/analytics repurposing,
and local-target-only testing.

## Honest limitations (the known floor)

- A **perfect human-like forgery from a fresh identity** can still clear the Pass puzzle.
  This is the fundamental limit of any accessible challenge; it is bounded by the
  attestation issuance rate and by folded engine risk, and the engine still **DENIES**
  the session as soon as any independent bot signal appears.
- **Anti-replay is best-effort.**
- The anti-injection **CSP is currently report-only** (violation telemetry), not an
  enforced block.
- **Admin-plane security** in the reference build depends on loopback binding and no
  auto-issued token; a production operator should front it with **mTLS/SSO**.
- The audit chain's tamper-evidence resists external tampering and enables post-hoc
  verification, but not an attacker with in-process/code control (production runs the
  witness and keys out-of-process/HSM).

## Reporting a security issue

Please report privately per [`SECURITY.md`](../../SECURITY.md) or
[`security.txt`](https://humanymous.net/.well-known/security.txt) — we run a coordinated
vulnerability disclosure process with published response targets and safe harbor.
