# Security Policy

humanymous is a defensive bot-detection engine. A tool that detects attackers is
itself a high-value target, so we treat vulnerability reports seriously and run a
**coordinated vulnerability disclosure (CVD)** process aligned with **ISO/IEC 29147**
(disclosure) and **ISO/IEC 30111** (handling).

## Supported versions

| Version | Supported |
|---------|-----------|
| latest `main` / newest tagged release | ✅ security fixes |
| older tagged releases | ❌ upgrade to the latest |

We ship from `main` and tag releases with [Semantic Versioning](https://semver.org/).
Security fixes land on the latest release line; there is no long-term back-port branch
at this stage of the project. Pin to a tag and watch releases to receive fixes.

## Reporting a vulnerability

**Please report privately — do not open a public issue for a security bug.**

1. Preferred: **GitHub Private Vulnerability Reporting** — the *Security → Report a
   vulnerability* button on the repository (creates a private advisory draft).
2. Or email **support@modoo.today** with subject `humanymous security`. If you need
   encryption, request our PGP key in the first message.

Please include: affected component (engine `:8443`, Gate edge `:8444` / admin `:8445`,
humanymous Pass, WASM/JS loader, audit chain), version/commit, a minimal reproduction,
and the impact you observed. A working proof-of-concept helps us triage faster.

## Our commitments (SLAs)

These are targets, made in good faith:

- **Acknowledge** your report within **~5 business days**.
- Send a **status update at least every ~30 days** while the report is open.
- Aim for a **coordinated fix and publication within ~90 days** of triage, sooner for
  actively-exploited issues. We will agree a disclosure date with you.
- On fix, publish a **GitHub Security Advisory (GHSA)** and request a **CVE** where the
  issue warrants one, crediting you unless you prefer to remain anonymous.

## Safe harbor

If you make a good-faith effort to follow this policy, we will not pursue or support
legal action against you for your research, and we consider it authorized. In scope:
**your own local instance** of humanymous only. **Out of scope / not authorized:**
testing against systems you do not own or operate, accessing or exfiltrating other
people's data, denial-of-service against shared infrastructure, physical or social-
engineering attacks, and anything that violates applicable law. The bundled red-team
catalog in `test/redteam/` is for validating *your own* detector — see the ethical
notice in the README.

## Known limitations (not vulnerabilities)

Some properties are inherent design trade-offs, documented rather than fixed. See the
[security audit report](docs/reference/security-audit.md) and the
[transparency report](docs/explanation/transparency-report.md) for the honest posture,
including: the detection floor (a perfect human-like forgery from a fresh identity can
still clear the Pass challenge), the report-only CSP, admin-plane security depending on
operator-network isolation until mTLS/SSO is provisioned, and best-effort anti-replay.

## Scope of this policy

This policy covers the code in this repository and the reference deployment at
`humanymous.net`. A machine-readable contact is published at
`https://humanymous.net/.well-known/security.txt`.
