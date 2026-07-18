# Versioning, release notes & SoT→user-doc derivation index

> Reference · Governance — maintainer page. Audience: humanymous Gate maintainers and doc authors.

> **Note:** This is a **maintainer-facing** page. It is the **only** page in the documentation set that may reference internal source-of-truth identifiers (`SoT-NN`). Every reader-facing page must not surface those identifiers. If you copy content from here into a reader-facing page, strip the `SoT-NN` references first.

This page explains how the humanymous Gate documentation set is versioned, how behavior changes reach a changelog, and how each user-facing page derives from one or more internal source-of-truth (SoT) specs. Use it to keep the docs from drifting when the engine or a SoT changes.

## Who this is for

- **Maintainers** deciding which pages to review after an engine release or a SoT change.
- **Doc authors** confirming which SoT a page derives from before editing it.

For voice, terminology, and formatting rules that govern every reader-facing page, see the [style guide](../style-guide.md).

## 1. Versioning

humanymous Gate exposes two distinct version stamps. Keep them separate — they answer different questions.

### Scoring policy version

The scoring policy carries a semantic version. The reference implementation ships policy version **`1.0.0`** (`internal/scoring/policy.go`). This version identifies the **decision rules** — the risk band thresholds (0–29 ALLOW, 30–69 CHALLENGE, 70–100 DENY; `ChallengeAt=30`, `DenyAt=70`) and the hard-rule overrides. When you change a threshold, add or retire a hard rule, or change how a verdict is reached, that is a scoring-policy behavior change and the policy version should move.

### Signed config version

Separately, `config_version` is a **signed HMAC hash of the effective policy** — the routes, rate limits, monitor state, and kill-switch state combined. It is:

- **stamped on every audit record**, and
- **exposed at `GET /__hmn/admin/policy`**.

Because it is derived from the effective configuration, an operator can read `config_version` after a change or upgrade and confirm that the policy now in force is the one they intended. Two nodes reporting the same `config_version` are enforcing the same effective policy; a changed `config_version` is evidence that the effective policy changed.

> **Tip:** When you document a policy or preset change, point operators at `GET /__hmn/admin/policy` (and the Policy view in the Ledger) so they can verify the new `config_version` took effect — do not ask them to infer it from behavior.

### Aligning docs to engine releases

Align the documentation version to the **engine release** it describes. When the engine cuts a release, the docs set that describes it should carry a matching version marker so a reader can tell which engine build a page was written against.

### Release notes / changelog

A release-notes changelog should surface **behavior changes** that affect operators, integrators, or end users. At minimum, call out:

- **New or retired hard rules** (for example, a new `HR-NN` → DENY/CHALLENGE rule).
- **Preset changes** — anything that alters what `off` / `monitor` / `balanced` / `strict` do, or the default route→preset mapping.
- **Threshold or escalation changes** — moved risk bands (`ChallengeAt` / `DenyAt`), changed ban-ladder steps, or changed rate-limit windows.

Each such entry should name the mechanism precisely and link the reader-facing page(s) that document it, using the derivation index below to find them.

> **TODO(verify):** Confirm the concrete location, filename, and format of the release-notes changelog in the repo (for example, `CHANGELOG.md` at repo root vs. a docs page). The brief does not specify where the changelog lives.

## 2. SoT → user-doc derivation index

Each user-facing page derives from one or more internal SoT specs. When a SoT changes, use this table to find every page that must be reviewed. Keep it current: adding a page or re-scoping one means updating its row here.

Paths are repo-relative from `docs/`.

### Published (P0 + P1)

| User doc | Derives from |
|---|---|
| `README.md` | SoT-29 |
| `start-here/integrator.md` | SoT-29 |
| `start-here/operator.md` | SoT-29 |
| `start-here/compliance-dpo.md` | SoT-29 |
| `start-here/evaluator.md` | SoT-29 |
| `style-guide.md` | SoT-29 |
| `concepts/how-gate-sees-a-request.md` | SoT-00 |
| `tutorials/quickstart-monitor-mode.md` | SoT-19, SoT-20 |
| `explanation/what-gate-is.md` | SoT-00, SoT-06 |
| `explanation/will-this-break-my-app.md` | SoT-19, SoT-21 |
| `explanation/where-gate-fits.md` | SoT-06 |
| `explanation/control-plane-and-bundle.md` | SoT-19, SoT-20 |
| `reference/cli-config-policy.md` | SoT-24, SoT-27 |
| `reference/hard-rules-verdicts.md` | SoT-05, SoT-25 |
| `reference/install-requirements.md` | SoT-19 |
| `reference/rbac-separation-of-duties.md` | SoT-24, SoT-28 |
| `reference/data-processing-inventory.md` | SoT-18, SoT-22 |
| `reference/production-vs-reference.md` | SoT-22, SoT-28 |
| `reference/security-disclosure.md` | SoT-06, SoT-29 |
| `runbooks/incident-runbooks.md` | SoT-25, SoT-27 |
| `runbooks/kill-switch-and-bans.md` | SoT-24, SoT-27 |
| `runbooks/erasure-crypto-shred.md` | SoT-18 |
| `help/why-am-i-seeing-this.md` | SoT-21, SoT-26 |
| `help/challenge-accessibility.md` | SoT-21, SoT-26 |
| `how-to/audit-console-tour.md` | SoT-26 |
| `how-to/deployment-policy-operations.md` | SoT-21, SoT-23, SoT-24, SoT-27 |
| `how-to/verify-audit-log.md` | SoT-18 |
| `how-to/self-validation-red-team.md` | SoT-04, SoT-25 |
| `how-to/detection-observatory.md` | SoT-30 |
| `how-to/observatory-tour.md` | SoT-30 |
| `explanation/observatory-architecture.md` | SoT-30 |
| `explanation/which-piece-am-i-using.md` | SoT-00, SoT-19, SoT-30 |
| `explanation/detection-engine-internals.md` | SoT-00, SoT-05 |
| `how-to/run-detection-engine.md` | SoT-00, SoT-30 |
| `how-to/extend-detection.md` | SoT-05 |
| `reference/red-team-catalog.md` | SoT-04 |
| `explanation/red-catalog-architecture.md` | SoT-04, SoT-07, SoT-12 |
| `how-to/write-a-red-profile.md` | SoT-04 |
| `reference/red-team-rules-of-engagement.md` | SoT-06, SoT-29 |
| `start-here/developer.md` | SoT-29 |
| `how-to/key-management.md` | SoT-22, SoT-28 |
| `how-to/upgrade-migration.md` | SoT-22 |
| `how-to/observability-siem.md` | SoT-25 |

### This batch (P2)

| User doc | Derives from |
|---|---|
| `how-to/troubleshooting-faq.md` | SoT-19, SoT-23 |
| `reference/deployment-cost-latency.md` | SoT-22, SoT-25 |
| `reference/standards-mapping.md` | SoT-18 |
| `reference/on-call-quick-reference.md` | SoT-25, SoT-27 |
| `explanation/dpia-companion.md` | SoT-18, SoT-22 |
| `reference/support-licensing.md` | SoT-29 |
| `reference/console-localization.md` | SoT-26, SoT-29 |
| `reference/versioning-derivation-index.md` (this page) | SoT-29 |

## 3. Change-control rule

The entire user-facing documentation set is derived from the internal source-of-truth specs **SoT-00 through SoT-30**, and it must not drift from them. Authority split: **SoT-00** governs the engine/behavior, **SoT-29** governs the user-facing doc surfaces, and **SoT-30** governs the Detection Observatory and its live-telemetry design. Where a page describes engine/behavior it defers to SoT-00; where it describes doc voice/structure it defers to SoT-29.

Two SoTs anchor the split of authority:

- **SoT-00 is the engine / behavior authority.** It governs what Gate actually does — how a request is scored, which layers apply, and how verdicts are reached. Pages describing behavior trace back to SoT-00 (directly or through a more specific SoT).
- **SoT-29 governs the user-facing surfaces** — voice, terminology, formatting, and the shape of the documentation set itself (including this page).

The rule is directional and simple:

> **A change to any SoT triggers a review of every user-facing page derived from it.**

When you change a SoT:

1. Look it up as a value in the derivation index above.
2. Review each page listed against it, and update any page whose content the change affects.
3. If the change alters behavior (a threshold, preset, hard rule, or escalation step), also add a release-notes entry per Section 1 and confirm the scoring policy version and/or `config_version` implications are documented.

When you add or re-scope a user-facing page, add or update its row in the derivation index so the mapping stays complete. A page with no SoT row, or a SoT change with no page review, is drift.
