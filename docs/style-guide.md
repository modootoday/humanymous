---
title: Documentation style guide
description: "Rules for clear, honest, accessible humanymous Gate documentation and product text."
---

# Documentation style guide

**Diátaxis quadrant:** Reference — apply these rules while writing or reviewing.
**Audience:** documentation authors, interface writers, reviewers, and maintainers.

This guide governs public documentation, interface text, command help, ordinary logs, examples, release notes, and generated language-model mirrors. The [glossary](reference/glossary.md) is the single source of truth for reader-facing terminology.

humanymous Gate is a reference implementation, not a production-hardened service. Public text says so anywhere a reader could otherwise infer a production guarantee.

## Voice

The product voice is calm, precise, candid, respectful, verifiable, and defensive-only.

### Calm

State the mechanism without drama.

- Do not use urgency theatre or victory language.
- Avoid exclamation marks and verbs such as “crushes”, “destroys”, or “slams”.
- Describe a blocked request as an operational outcome, not a triumph.

### Precise

Name the mechanism in words a reader can understand.

- Name the observed evidence, resulting verdict, route policy, and edge action.
- Do not substitute an internal number for an explanation.
- Do not anthropomorphize the software with “thinks”, “knows”, or “figures out”.

### Candid

State the limit beside the capability.

- Pair detection claims with false-positive and challenge-friction risks.
- State the coherent-automation detection boundary: internally consistent real-browser or human-assisted automation cannot be reliably separated from legitimate traffic by client and network signals alone.
- Distinguish Core measurements from Gate measurements.
- Label operator-supplied production responsibilities.

### Respectful

Treat challenged or blocked traffic as belonging to real people until evidence and review establish otherwise.

- Use “automated”, “not verified”, or a neutral description of the request.
- Do not call a visitor an attacker, hacker, robot, or bad actor.
- Threat-model documents may name an abstract capability class, but should not assign that label to ordinary traffic.

### Verifiable

Give the reader a way to check the statement.

- Prefer a command, endpoint, expected log line, or test over a bare assertion.
- Label performance, detection, and user-impact numbers as reference-measured and name the setup.
- Do not invent a number where the reference has not measured one.

### Defensive-only

Every validation or probing instruction targets a deployment the reader owns or is authorized to test.

- Do not frame techniques as testing third-party systems.
- Do not provide a free-form target path in a self-validation tool.

## Public terminology boundary

Reader-facing text uses full concepts from the glossary. The following remain internal:

- internal specification and plan numbers;
- numbered implementation phases and workstream labels;
- numbered detection-stage codes;
- numbered threat-tier codes;
- numbered enforcement-rule identifiers;
- raw dotted signal identifiers;
- private mutation-class letters;
- internal source type names used only by the implementation.

These identifiers may remain in:

- internal specifications and plans;
- developer comments and tests;
- exact compatibility fields in an application programming interface, raw event, or export;
- commands where the reader must type the literal value.

When an exact machine identifier is necessary:

1. Write the plain-language meaning first.
2. Put the exact literal in code formatting.
3. Keep it out of headings, navigation, buttons, default badges, tooltips, help prose, and ordinary logs.
4. In an interface, place it under technical details or raw export rather than using it as the primary label.

Do not teach internal identifiers as product vocabulary. The glossary defines concepts, not code coordinates.

## Acronyms and protocol names

Spell out a term on first use on each page. A standardized acronym may follow in parentheses only when a reader needs to recognize the external standard or exact protocol name.

Examples:

- role-based access control;
- data protection impact assessment;
- security information and event management system;
- write-ahead log;
- hash-based message authentication code;
- mutual Transport Layer Security;
- Content Security Policy;
- Server-Sent Events;
- Chrome DevTools Protocol;
- Classless Inter-Domain Routing.

Product-private abbreviations are not reused after expansion. Write “request-integrity token” and “proof-of-work challenge” in full.

Flags, environment variables, header names, cookie names, metric names, and application programming interface fields remain exact and use code formatting.

## Canonical product terms

Use the [glossary](reference/glossary.md) form.

| Use | Avoid |
|---|---|
| humanymous | capitalization variants |
| humanymous Gate on first mention; Gate thereafter | “the gateway” as the product name |
| Core detection engine | implying Core and Gate have identical coverage |
| Ledger | dashboard, admin panel |
| Detection Observatory | playground as the public product name |
| origin application | backend and upstream used interchangeably |
| verdict: ALLOW, CHALLENGE, or DENY | one ambiguous “blocked” state |
| risk score | trust score, bot percentage |
| enforcement rule | a bare internal rule number |
| seven-stage detection pipeline | numbered stage shorthand |
| coherent-automation detection boundary | a numbered threat ceiling |
| runtime settings | implementation type names |
| protection-increasing or protection-reducing change | private mutation-class letters |
| network evidence policy | internal all-caps policy labels |
| effective configuration version | a raw field name as the heading |
| built-in defaults remain active | internal freeze shorthand |
| cryptographic erasure | delete or wipe when describing the formal control |
| pseudonymous | anonymous |
| tamper-evident | tamper-proof |
| dual-control, naming the second role | two-man rule, four-eyes |
| reference implementation and production responsibility | unexplained maturity shorthand |

## Context-specific register

| Context | Register | Rule |
|---|---|---|
| Tutorial | Encouraging, concrete, linear | One working path with expected output. |
| How-to | Direct, goal-first | Assume competence; link concepts instead of reteaching them. |
| Reference | Flat, exhaustive, neutral | Prefer tables and exact values. |
| Runbook | Terse, numbered, unambiguous | One action per step; state blast radius and rollback. |
| Explanation | Measured and reasoned | Explain trade-offs and limits. |
| Privacy and compliance | Formal, precise, cited | Describe controls, not legal conclusions. |
| Operator interface | Brief and actionable | Plain label first; technical details second. |
| Blocked-user help | Warm, non-technical | No detection details; give a usable next action. |

Aim for Grade 9–11 prose. Privacy and legal material may be more formal where precision requires it.

## End-user and operator boundary

### End-user text

A challenged or blocked visitor may see:

- a plain statement of what happened;
- whether retrying later may help;
- an opaque incident handle when the deployment actually provides one;
- the site operator's support path;
- an accessible alternative when a challenge is present.

The visitor must not see:

- internal rule or signal identifiers;
- a risk score;
- browser or network fingerprint details;
- implementation or specification references;
- language asserting that the visitor is automated.

### Operator text

Operator interfaces use full rule names, full signal descriptions, clear mode names, and the effective configuration version. Raw identifiers belong in technical details and exports.

Do not promise controls the reference does not provide. Examples:

- the default reference block response does not provide a usable opaque incident handle in every case;
- the current reference Gate does not route every challenge to a solvable Pass;
- the reference witness is local, not independently operated;
- the reference retention tiers are not physically enforced;
- kill-switch and approval state are node-local until shared-state work lands.

## Honest-security gate

A public change is not ready when any item below fails.

- [ ] Capability and limit appear together.
- [ ] Core and Gate are not presented as having identical evidence or measured results.
- [ ] No banned absolute claim appears.
- [ ] No fear-selling or unquantified scale claim appears.
- [ ] No internal coordinate label appears in reader prose or primary interface text.
- [ ] Every acronym is expanded on first use.
- [ ] Every operational claim has a verification path where practical.
- [ ] Challenge friction and denial are reported separately.
- [ ] The coherent-automation detection boundary remains explicit.
- [ ] Self-validation targets only an owned or authorized deployment.
- [ ] Reference behavior and production responsibility are distinct.

Never ship: “unhackable”, “blocks all bots”, “one hundred percent”, “zero false positives”, “bulletproof”, “tamper-proof”, “military-grade”, or “makes you compliant”.

Use bounded verbs: reduces, raises the cost of, observes, flags, challenges, refuses, supports.

## Accessibility and inclusive language

Copy-level Web Content Accessibility Guidelines version 2.2, level AA is a merge requirement.

- Use allowlist and denylist.
- Use primary and replica.
- Use person-first, non-accusatory language.
- Do not say “click here”; use meaningful link text.
- Do not rely on direction or color alone.
- Give every explanatory image meaningful alternative text.
- Use sentence-case headings, one top-level heading, and no skipped heading levels.
- Never gate a challenge on motor richness, speed, pressure, tremor, or pointer precision.
- State a real time limit honestly and preserve work when a verification token must be renewed.

## Admonitions

Use only:

- `> **Note:**` for useful context;
- `> **Tip:**` for an optional shortcut;
- `> **Important:**` for a condition that changes the result;
- `> **Warning:**` for destructive, irreversible, or broad-impact consequences.

Cryptographic erasure always carries an irreversibility warning. A kill switch always states its actual node or fleet scope.

## Formatting

- Put the Diátaxis quadrant and audience directly below the title.
- Use one quadrant per page.
- Use sentence-case headings.
- Put commands and exact identifiers in code formatting.
- Use one command per fenced block and omit a leading shell prompt.
- Write placeholders as `<descriptive-name>` and explain who supplies the value.
- Keep a fact in one canonical page and link to it elsewhere.
- Regenerate language-model mirrors after source documentation changes; do not edit generated mirrors by hand.

## Review examples

**Internal coordinate replaced with a mechanism**

- Before: “A numbered enforcement rule matched.”
- After: “The headless browser also exposed a second automation indicator, so the enforcement rule raised the verdict to DENY.”

**Detection stage replaced with a full name**

- Before: “The cross-check stage disagreed with the network stage.”
- After: “The browser's user-agent claim disagreed with the protocol implementation observed by the server.”

**Threat tier replaced with its boundary**

- Before: “The highest tier is unsolved.”
- After: “Internally consistent real-browser or human-assisted automation cannot be reliably separated from legitimate traffic by client and network signals alone.”

**Blocked-user text**

- Before: “Automation detected.”
- After: “We could not complete this request. If you think this is a mistake, contact the site operator and quote the reference shown on this page.”

**Reference honesty**

- Before: “Gate scores all browser and network evidence.”
- After: “Core measures the full reference pipeline. The current Gate enforces traffic with a narrower set of browser, header, and source observations.”
