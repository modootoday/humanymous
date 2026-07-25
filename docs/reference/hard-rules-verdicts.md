---
title: Verdicts, enforcement rules, and diagnostic signals
description: "Reference tables for score-based verdicts, Core enforcement rules, Gate edge protections, and diagnostic signal output."
---

# Verdicts, enforcement rules, and diagnostic signals

**Diátaxis quadrant:** Reference — look up an outcome or predicate.
**Audience:** operators, integrators, evaluators, and developers.

This page names rules by their meaning. Raw event and application programming interface payloads retain legacy technical identifiers for compatibility, but those identifiers are not a safe global vocabulary: some current Core and Gate identifiers overlap while referring to different predicates. Always use the plane and descriptive name together when reading raw data.

humanymous Gate is a reference implementation. The tables describe current source behavior, not a universal policy.

## Score-based verdicts

| Verdict | Built-in score range | Meaning | Gate action |
|---|---:|---|---|
| ALLOW | below 30 | Active policy permits the request. | Forward to the origin unless an attested route requires another proof. |
| CHALLENGE | 30 through 69 | Additional verification is required. | Do not contact the origin until verification succeeds. |
| DENY | 70 and above | Active policy refuses the request. | Stop before the origin. |
| Unknown | no usable verdict | Evidence is not yet available. | Fail open or fail closed according to route policy and method. |

Runtime Settings can change the challenge and denial thresholds. Treat 30 and 70 as built-in defaults.

An enforcement rule can raise the score-based verdict. Rules are evaluated in a fixed order; the first matching rule whose mode is enforce wins. A matching rule in monitor mode is recorded as a shadow result and evaluation continues, so monitoring one heuristic cannot hide a later high-confidence match.

## Unknown-state behavior

Gate fails open only for safe methods on routes that do not require strict handling. It fails closed when:

- the route requires synchronous scoring or strict handling; or
- the method can change state, including POST, PUT, PATCH, and DELETE.

Every Gate enforcement action is sent to the audit sink before it takes effect.

## Proof-of-work upgrade

A completed proof-of-work challenge can upgrade a score-based CHALLENGE to ALLOW. It cannot override a rule-promoted verdict because it demonstrates computation, not that a person is present.

The current reference Gate does not provide a complete, user-solvable proof-of-work or Pass path for every challenged visitor. Do not describe CHALLENGE as recoverable in a deployment unless that path is wired and tested.

## Attested-route demotion

On a route using the `attested` preset, an otherwise allowed request must present accepted possession evidence or a valid step-up proof. Without one, Gate changes the outcome to CHALLENGE, never directly to DENY.

The attestation floor prices access to a high-value route. It does not detect coherent real-browser or human-assisted automation.

## Core enforcement rules

Core evaluates the following rules inside the shared scoring engine. “Heuristic” means the predicate can match legitimate traffic and therefore raises the verdict only to CHALLENGE.

| Public rule name | Predicate | Outcome | Confidence guidance |
|---|---|---|---|
| Hard automation artifact | Selenium, Puppeteer, Playwright, or Phantom-style artifact reported as confirmed automation | DENY | High confidence |
| Browser and network-engine mismatch | Browser user-agent claim disagrees with both the connection-negotiation and Hypertext Transfer Protocol version 2 implementations | DENY | High confidence when Core directly observed the client connection |
| Synthetic event plus browser-driver evidence | Untrusted events occur with WebDriver or browser-control evidence | DENY | High confidence |
| WebDriver concealment | WebDriver state occurs with a modified native getter | DENY | High confidence |
| Request-integrity failure plus protocol mismatch | Request-integrity authentication fails and the browser claim disagrees with the observed connection implementation | DENY | High confidence |
| Runtime hook plus automation evidence | Runtime hooks or prototype changes occur with WebDriver or browser-control evidence | DENY | High confidence |
| Headless browser plus another automation indicator | A headless user-agent token occurs with WebDriver, a chromeless window, or modified native code | DENY | High confidence |
| Stealth browser modification | A modified native getter occurs in a chromeless or headless environment | DENY | High confidence |
| Browser-control leak plus automation evidence | Browser-control protocol evidence occurs with another automation indicator | DENY | High confidence |
| Disabled developer console plus automation evidence | The browser console is disabled and another chromeless or automation indicator is present | DENY | High confidence |
| Missing browser evidence | No browser-side report is present | CHALLENGE | Heuristic; can affect script-blocking or unsupported clients |
| Request-body integrity failure | A request-integrity token fails header, body, or keyed-authentication validation | DENY | High confidence |
| In-session protocol-fingerprint rotation | The encrypted-connection implementation changes within one session | DENY | High confidence when Core directly observed the client connection |
| Browser claim without execution evidence | A request claims a browser but supplies no browser execution evidence | DENY | High confidence only when the report provenance supports the predicate; Gate's narrower collector is a documented limitation |
| Automated-browser interaction signature | A click without an approach trajectory occurs with another machine-interaction or browser-control indicator | DENY | High confidence for the combined predicate; individual behavior remains accessibility-sensitive |
| Cross-session correlation | One fingerprint rotates across networks, fingerprint churn follows a proxy, or proof-of-work completes implausibly quickly | DENY | High confidence for the combined predicate; network policy can monitor correlation residuals |
| Hypertext Transfer Protocol version 2 denial of service | Rapid Reset or continuation-frame flooding is observed | DENY | High confidence |
| Multi-axis identity rotation | User agent changes together with connection or source-address rotation | DENY | High confidence |
| Missing or replayed request-integrity token | An application programming interface call lacks the required token or reuses a stale one | CHALLENGE | Heuristic |
| No interaction observed | No interaction occurs during the observation window | CHALLENGE | Heuristic; can match legitimate reading, assistive technology, or non-pointer use |
| Datacenter-network browser | A consistent browser arrives from a datacenter network | CHALLENGE | Heuristic |
| Advanced browser residual | Audio-rate, graphics-processor, or mobile-versus-desktop evidence remains inconsistent near the detection boundary | CHALLENGE | Heuristic |
| Chromium-container residual | A Chrome claim lacks Widevine, media devices, and voices in the specific combined pattern | CHALLENGE | Heuristic |
| Network residual policy | Enforced policy selects residual evidence involving proxy hops, virtual private networks, Tor, or source spoofing | CHALLENGE | Operator-controlled and heuristic; Transmission Control Protocol residuals never auto-enforce |

Core uses the same ordered predicate table for the verdict and the Detection Observatory trace. This prevents the explanatory view from evaluating a second set of rules.

## Gate edge protections

Gate edge protections run outside the scoring engine. Runtime Settings controls them by module name rather than by Core rule identifier.

| Public protection name | Predicate or condition | Action | Monitoring behavior |
|---|---|---|---|
| Active ban | Source address, network range, or fingerprint has an active ban | Refuse before scoring or origin contact | Records what would have been refused |
| Hard request-rate breach | Rolling source or fingerprint bucket exceeds the hard limit | Refuse and apply the ban ladder | Records the breach without refusing |
| Request-smuggling protection | Ambiguous message framing such as conflicting length and transfer-encoding headers | Refuse before routing | Records what would have been refused |
| Trusted-header forgery protection | A client supplies an internal trust or forwarding header reserved for Gate | Refuse and strip the forged input | Records what would have been refused |
| Verdict-token replay and forgery protection | Trust token is forged, expired, lifted, or bound to different client characteristics | Refuse | Records the failed validation |
| Connection-upgrade protection | WebSocket or Server-Sent Events upgrade lacks a prior fingerprint-bound verdict | Refuse the upgrade | Records what would have been refused |
| Decision-probing sweep protection | One fingerprint creates enough near-identical sessions to map decision boundaries | Refuse after the threshold and apply a constant timing floor | Records the sweep |
| Single-use proof replay protection | A beacon or proof nonce is reused | Reject the proof | Records the replay |
| Response-injection safety guard | An eligible response is compressed unexpectedly, hostile, or larger than the bounded injection buffer | Pass the origin response through without injection | Records the skipped injection |
| Direct-origin bypass protection | A request reaches the protected origin without Gate's authenticated origin header | Origin returns misdirected request | Enforced at the origin boundary |

The response-injection safety guard is deliberately fail open for origin content. It prevents Gate from buffering an unbounded response; it is not a blocked-client verdict.

## Machine-identifier collision

Current source contains overlapping legacy number ranges between recent Core rules and older Gate audit labels. Detection behavior is frozen, so this documentation change does not renumber predicates.

Operational consequences:

- do not interpret a raw rule identifier without the event type and plane;
- use `engine` or `edge` in exported records and security-event-management parsing;
- show the descriptive name in the Ledger;
- keep raw identifiers in technical details and exports only;
- do not build a new policy or alert keyed only to the unqualified legacy number.

## Diagnostic signals

Raw score traces contain dotted signal identifiers because they are machine contracts. Reader-facing interfaces should translate them into descriptions.

| Description | Evidence family |
|---|---|
| Browser-control proxy leak | Static client inspection |
| WebDriver state exposed | Static client inspection |
| Graphics renderer and parameter mismatch | Client fingerprinting |
| Click with no approach trajectory | Interaction analysis |
| Machine-like burst and silence cadence | Interaction analysis |
| Rapid Reset frame pattern | Network and protocol inspection |
| One fingerprint rotating across networks | Cross-session correlation |
| Request body does not match its integrity token | Request integrity |
| Proof-of-work completed | Challenge state |

Cross-check identifiers use a separate machine namespace. The public meanings are:

- user-agent claim versus connection-negotiation implementation;
- user-agent claim versus Hypertext Transfer Protocol version 2 implementation;
- browser claim versus browser-side execution evidence.

## Reading a rule spike

- For a high-confidence combined predicate, verify representative records and the collection plane before treating the rise as automated traffic.
- For a heuristic, investigate legitimate client changes, accessibility impact, script blocking, network topology, and challenge friction first.
- A missing rule after a Settings change may indicate monitor mode rather than the predicate disappearing.
- A network rule falling to zero may indicate encrypted-connection re-termination, not an improvement.

## Related

- [Glossary](glossary.md)
- [How humanymous evaluates a request](../concepts/how-gate-sees-a-request.md)
- [Runtime policy and configuration](cli-config-policy.md)
- [Incident runbooks](../runbooks/incident-runbooks.md)
- [Supported topologies](supported-topologies.md)
