---
title: humanymous Gate documentation
description: "Documentation for the humanymous Core detection engine, Gate reverse proxy, Ledger operator console, and local Detection Observatory."
---

# humanymous Gate documentation

**Diátaxis quadrant:** Navigation hub.
**Audience:** everyone arriving for the first time.

humanymous Gate is an Apache-2.0 reference implementation for reverse-proxy enforcement, request metering, and tamper-evident operational audit. The related Core detection engine is the fuller development and measurement surface. Core and Gate share scoring code but do not collect identical evidence.

> **Important:** Core's measurements are not Gate's measurements. Read [Which piece am I using?](explanation/which-piece-am-i-using.md) before interpreting detection results.

The [glossary](reference/glossary.md) is the single source of truth for product terms. Public pages use full concept names instead of internal specification numbers, numbered rule codes, numbered detection stages, or numbered threat tiers.

## Start by role

- [Integrator](start-here/integrator.md) — place Gate in front of an application and roll it out safely.
- [Operator](start-here/operator.md) — use the Ledger, runtime Settings, bans, approvals, and on-call procedures.
- [Data protection reviewer](start-here/compliance-dpo.md) — evaluate pseudonymization, audit, erasure, retention, and their limits.
- [Evaluator](start-here/evaluator.md) — assess capabilities, evidence, topology requirements, and production responsibilities.
- [Developer](start-here/developer.md) — inspect Core, extend detection, and run defensive self-validation.

## Tutorial

- [Quickstart in monitor mode](tutorials/quickstart-monitor-mode.md) — start Gate without enforcing verdicts.

## How-to guides

- [Deployment and policy operations](how-to/deployment-policy-operations.md)
- [Configure attested routes](how-to/configure-attested-routes.md)
- [Configure credential verifiers](how-to/configure-credential-verifiers.md)
- [Tour the Ledger](how-to/audit-console-tour.md)
- [Verify the audit log](how-to/verify-audit-log.md)
- [Manage keys, rotation, and recovery](how-to/key-management.md)
- [Upgrade and migrate](how-to/upgrade-migration.md)
- [Integrate observability and security event management](how-to/observability-siem.md)
- [Use the Detection Observatory](how-to/detection-observatory.md)
- [Tour the Detection Observatory](how-to/observatory-tour.md)
- [Run the Core detection engine](how-to/run-detection-engine.md)
- [Extend detection](how-to/extend-detection.md)
- [Operate the behavioral model](how-to/operate-behavioral-model.md)
- [Run defensive self-validation](how-to/self-validation-red-team.md)
- [Write a validation profile](how-to/write-a-red-profile.md)
- [Troubleshoot an integration](how-to/troubleshooting-faq.md)
- [Configure certificates](how-to/https-tls-certificates.md)
- [Verify a release image](how-to/verify-the-image.md)
- [Cut a release](how-to/cut-a-release.md)

## Reference

- [Glossary](reference/glossary.md)
- [Command line, configuration, and route policy](reference/cli-config-policy.md)
- [Verdicts, enforcement rules, and diagnostic signals](reference/hard-rules-verdicts.md)
- [Install requirements and supported platforms](reference/install-requirements.md)
- [Roles, separation of duties, and dual-control](reference/rbac-separation-of-duties.md)
- [Data-processing inventory](reference/data-processing-inventory.md)
- [Gate-to-origin contract](reference/gate-origin-contract.md)
- [Supported topologies](reference/supported-topologies.md)
- [Reference behavior and production responsibilities](reference/production-vs-reference.md)
- [Deployment cost, latency, and footprint](reference/deployment-cost-latency.md)
- [On-call quick reference](reference/on-call-quick-reference.md)
- [Standards and regulatory mapping](reference/standards-mapping.md)
- [Defensive validation catalog](reference/red-team-catalog.md)
- [Defensive validation rules of engagement](reference/red-team-rules-of-engagement.md)
- [Security audit](reference/security-audit.md)
- [Security disclosure](reference/security-disclosure.md)
- [Support, licensing, and notices](reference/support-licensing.md)
- [Console localization](reference/console-localization.md)
- [Documentation style guide](style-guide.md)

## Explanation

- [How humanymous evaluates a request](concepts/how-gate-sees-a-request.md)
- [What Gate is and is not](explanation/what-gate-is.md)
- [Which piece am I using?](explanation/which-piece-am-i-using.md)
- [Where Gate fits](explanation/where-gate-fits.md)
- [Will this break my app?](explanation/will-this-break-my-app.md)
- [The control plane and injected bundle](explanation/control-plane-and-bundle.md)
- [Inside the detection engine](explanation/detection-engine-internals.md)
- [How the self-correcting behavioral model works](explanation/self-correcting-behavioral-model.md)
- [Inside the Detection Observatory](explanation/observatory-architecture.md)
- [Validation catalog architecture](explanation/red-catalog-architecture.md)
- [Transparency report](explanation/transparency-report.md)
- [Data Protection Impact Assessment companion](explanation/dpia-companion.md)
- [How this reference was built](explanation/how-this-was-built.md)

## Runbooks

- [Incident response](runbooks/incident-runbooks.md)
- [Kill switch and bans](runbooks/kill-switch-and-bans.md)
- [Cryptographic erasure](runbooks/erasure-crypto-shred.md)

## End-user help

- [Why am I seeing this?](help/why-am-i-seeing-this.md)
- [Challenge accessibility](help/challenge-accessibility.md)
- [Frequently asked questions](help/faq.md)

## Boundaries to understand first

- The current Gate collects less browser and network evidence than Core.
- An intermediary that terminates and recreates encryption hides the client's original network fingerprint.
- Internally consistent real-browser or human-assisted automation cannot be reliably separated from legitimate traffic by client and network signals alone.
- The current reference Gate does not route every challenge through a complete, user-solvable Pass flow.
- The reference audit witness is local, retention tiers are not physically enforced, and several multi-node administrative controls remain node-local.
- The bundled validation catalog targets Core and cannot establish a population-wide false-positive rate.

## Defensive-only use

Run validation and probing only against a deployment you own or are authorized to test. The repository does not provide a third-party targeting workflow.
