# humanymous Gate documentation

Navigation hub · Audience: everyone arriving for the first time.

Start here to find the right path for your role, then use the doc map to jump to any page in the set.

## What is humanymous Gate?

humanymous Gate is a reverse-proxy enforcement layer that sits between your end users and your origin app. It terminates TLS, streams a detection bundle into the HTML, and scores each request across seven layers (L1–L7) — static client signals, fingerprint, client integrity, behavior, network/protocol, cross-check consistency, and scoring. The result is not a binary bot/human flag: each request gets a risk score from 0 to 100, which resolves to one of three verdicts — ALLOW, CHALLENGE (an accessible proof-of-work interstitial), or DENY — with hard rules able to promote a verdict on high-confidence evidence. Gate enforces that verdict at the edge and writes every decision to a tamper-evident audit log before it takes effect. It fronts your origin app and does not control it.

> **Note:** This repository is a reference implementation, not a production-hardened build. See the "Reference implementation" note below.

## Start here — pick your path

Five role-based landing pages, each a short orientation into the rest of the docs:

- **[Integrator](start-here/integrator.md)** — you are putting Gate in front of an app: topology, wiring, and rollout.
- **[Operator](start-here/operator.md)** — you run Gate day to day: the Ledger, bans, policy, and on-call.
- **[Compliance / DPO](start-here/compliance-dpo.md)** — you own privacy and data handling: pseudonymization, retention, and right-to-erasure.
- **[Evaluator](start-here/evaluator.md)** — you are assessing what Gate does and does not do before adopting it.
- **[Developer (Red/Blue)](start-here/developer.md)** — you want to understand and extend the detection engine and its local test catalog.

> **New here?** [Which piece am I using?](explanation/which-piece-am-i-using.md) maps the three surfaces — the standalone detection engine (:8443), the Gate proxy (:8444/:8445), and the Detection Observatory — so you never confuse two binaries.

## Doc map

The documentation set, grouped by Diátaxis quadrant.

### Tutorial

- **[Quickstart — monitor mode (30 min)](tutorials/quickstart-monitor-mode.md)** — stand up Gate in monitor mode and watch it score real traffic without enforcing anything.

### How-to

- **[Deployment & policy operations](how-to/deployment-policy-operations.md)** — task recipes: TLS/certs, origin cloaking, admin listener + RBAC, moving a route from monitor to enforce, rate limits and bans.
- **[Configure attested routes (the attestation floor)](how-to/configure-attested-routes.md)** — price the ALLOW on high-value routes: mark a route `attested`, its shared-key and cookie-jar preconditions, and how a real user clears the floor with a Pass or possession.
- **[Configure credential verifiers](how-to/configure-credential-verifiers.md)** — wire WebAuthn, Privacy Pass, and Web Bot Auth so verifiable clients skip the friction and fast-path past the attestation floor.
- **[Ledger tour](how-to/audit-console-tour.md)** — a guided walk of the six console views (Overview, Integrity, Sessions, Rate Limits & Bans, Policy, Compliance).
- **[Verify the audit log](how-to/verify-audit-log.md)** — how to check the tamper-evident log without trusting the operator, and what each integrity-mismatch class means.
- **[Self-validation: red-team your own deployment](how-to/self-validation-red-team.md)** — measure your own detection and false-positive rate with the bundled harness (defensive-only, local).
- **[Key management, rotation & recovery](how-to/key-management.md)** — the keystore, signing/HMAC/vault keys, ephemeral-vs-persistent identity, and recovery blast radius.
- **[Upgrade, migration & zero-downtime](how-to/upgrade-migration.md)** — the safe upgrade posture for a single node, and what multi-node rolling upgrades require.
- **[Cut a release](how-to/cut-a-release.md)** — SemVer `v*` tags drive ghcr.io images and a git-cliff changelog generated from Conventional Commits; the version-bump rules and the release flow.
- **[Observability & SIEM integration](how-to/observability-siem.md)** — the audit stream, integrity status, KPIs, and how to bridge to a SIEM.
- **[Watch detection happen live: the Detection Observatory](how-to/detection-observatory.md)** — the local-only, dev-gated page that streams the L1→L7 pipeline as it scores a session.
- **[A guided tour of the Detection Observatory](how-to/observatory-tour.md)** — an annotated screenshot walkthrough of the page before you run it.
- **[Integration troubleshooting & FAQ](how-to/troubleshooting-faq.md)** — symptom → cause → fix for the things that actually go wrong during integration.

### Reference

- **[CLI, config & per-route policy](reference/cli-config-policy.md)** — every flag, environment variable, policy preset, and the route match model.
- **[Hard rules, verdicts & signal IDs](reference/hard-rules-verdicts.md)** — the engine (HR-1..21) and Gate (HR-22..30) rule planes, verdict thresholds, and the lowercase l{n}.{group}.{item} signal namespace.
- **[Install, requirements & supported platforms](reference/install-requirements.md)** — toolchain, build, ports, and dependencies.
- **[RBAC, separation-of-duties & dual-control](reference/rbac-separation-of-duties.md)** — the role×capability matrix and which actions need a distinct committer.
- **[Data processing & personal-data inventory](reference/data-processing-inventory.md)** — what is observed vs stored, pseudonymization, retention tiers (RoPA-ready).
- **[Supported topologies (read before deploying)](reference/supported-topologies.md)** — which detection layers are active in each topology; the raw-TLS-termination requirement, the Core-vs-Gate split, and why a fronting CDN silently disables the network plane.
- **[Production-ready vs reference (prod-delta)](reference/production-vs-reference.md)** — component-by-component status and the local↔production checklist.
- **[Security vulnerability disclosure policy](reference/security-disclosure.md)** — how to report a flaw in Gate itself, with a security.txt template.
- **[Deployment cost, latency & footprint](reference/deployment-cost-latency.md)** — what adds per-request cost, the defined resource bounds, and how to measure your own numbers.
- **[Standards & regulatory mapping](reference/standards-mapping.md)** — control→requirement traceability (GDPR/PIPA/RFCs) and the audit-response evidence pack.
- **[On-call quick-reference & KPI thresholds](reference/on-call-quick-reference.md)** — a one-screen cheat sheet: verdicts, top hard rules, the levers, and the ban ladder.
- **[Support, licensing & OSS notices](reference/support-licensing.md)** — project-license status, third-party dependencies, and where to get help.
- **[Console localization & product-surface language](reference/console-localization.md)** — what is English today and what localization is a deployment responsibility.
- **[Versioning & SoT→doc derivation index](reference/versioning-derivation-index.md)** — maintainer governance: versioning, release notes, and which spec each doc derives from.
- **[Documentation style guide](style-guide.md)** — voice, terminology, and formatting rules for doc authors.

### Explanation

- **[Concepts & glossary — how Gate sees a request](concepts/how-gate-sees-a-request.md)** — the L1–L7 pipeline and the vocabulary the rest of the docs use.
- **[What Gate is (and is not)](explanation/what-gate-is.md)** — scope, the attacker-tier ceiling, and honest boundaries.
- **[Will this break my app?](explanation/will-this-break-my-app.md)** — safety, fail-open behavior, and a staged rollout from monitor to enforce.
- **[Where Gate fits](explanation/where-gate-fits.md)** — vs WAF / CDN bot manager / CAPTCHA, with the threat model and honest limitations.
- **[The control plane and the injected bundle](explanation/control-plane-and-bundle.md)** — the /__hmn/ namespace, streaming injection, and avoiding CSP/routing collisions.
- **[DPIA companion & retention/lifecycle](explanation/dpia-companion.md)** — a risks-and-mitigations input to your Data Protection Impact Assessment, and the retention model.

### Runbook

- **[Incident runbooks (on-call)](runbooks/incident-runbooks.md)** — step-by-step responses for the situations an operator meets in production.
- **[Kill switch & bans](runbooks/kill-switch-and-bans.md)** — blast radius, the ban ladder, apply/escalate/lift, and the dual-control gates.
- **[Right-to-erasure (crypto-shred) runbook](runbooks/erasure-crypto-shred.md)** — the DPO-gated, dual-control cryptographic erasure procedure.

### End-user help

- **[Why am I seeing this?](help/why-am-i-seeing-this.md)** — plain-language help and appeal path for a blocked or challenged visitor.
- **[Challenge accessibility statement](help/challenge-accessibility.md)** — the accessible-challenge commitment and what to do if the check is not usable.

### Developer / advanced (Red/Blue)

The depth tier for developers who want to understand and extend the detection engine and its local test catalog. Start at **[Developer (Red/Blue)](start-here/developer.md)**.

- **[Which piece am I using?](explanation/which-piece-am-i-using.md)** — the standalone engine (:8443) vs the Gate proxy (:8444/:8445) vs the Observatory, in one screen.
- **[Run the standalone detection engine](how-to/run-detection-engine.md)** — build and run `bin/server.exe` directly.
- **[Inside the detection engine](explanation/detection-engine-internals.md)** — the Signal schema, the noisy-OR + LayerCap + dedup math, the hard-rule table, and the ScoreTrace.
- **[Extend the detector](how-to/extend-detection.md)** — add a signal, a cross-check, or a hard rule without regressing the low-false-positive principle.
- **[Inside the Detection Observatory](explanation/observatory-architecture.md)** — the live-telemetry transport and the complete, enumerated safety model.
- **[Red-team rules of engagement](reference/red-team-rules-of-engagement.md)** — the defensive, local-only, self-target-only boundary every Red page rests on.
- **[Red-team catalog reference](reference/red-team-catalog.md)** — the profile contract, the 47-profile catalog, the raw-protocol attacks, and how a verdict becomes TP/FN/FP/TN.
- **[The Red catalog architecture](explanation/red-catalog-architecture.md)** — how the bundled profiles and the raw-protocol client are built.
- **[Write or extend a Red-team profile](how-to/write-a-red-profile.md)** — author a profile and register it in all three places.
- **[How this reference implementation was built](explanation/how-this-was-built.md)** — the method behind the system: spec-first design, adversarial self-validation, and real-network validation.

## Defensive-only

These docs describe protecting your own deployment. Any red-team, validation, or probing guidance targets a Gate instance you operate — never a third party's systems.

## Reference implementation, not production-hardened

This build is a reference implementation meant to show how the pieces fit together. Some capabilities are documented as reserved or as prod-delta (for example, ACME/bring-your-own certificates and a dedicated false-positive/appeal-queue view). Where a doc describes something the reference binary does not ship, it says so. Do not treat this build as production-ready without the production-hardening work called out in the relevant pages.
