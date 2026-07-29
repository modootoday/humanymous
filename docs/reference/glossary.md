---
title: Glossary
description: "Canonical plain-language definitions for humanymous Gate, its evidence, verdicts, controls, audit trail, deployment model, and limits."
---

# Glossary

**Diátaxis quadrant:** Reference — look up a term without reading the documentation from start to finish.
**Audience:** integrators, operators, evaluators, privacy reviewers, support teams, and documentation authors.

This page is the single source of truth for reader-facing terminology across humanymous documentation, user interfaces, command help, logs, examples, and release notes. Other pages link here instead of maintaining their own glossary.

The public vocabulary names concepts in full. Internal specification numbers, implementation-stage labels, rule numbers, numbered detection-stage codes, and numbered threat tiers are not reader-facing terms. Exact field names, flags, headers, metrics, and other machine identifiers may still appear in code formatting when a reader must type or inspect them.

humanymous Gate is a reference implementation, not a production-hardened service. Definitions distinguish what the reference build does today from controls an operator must add for production.

## Product and execution surfaces

**humanymous.** The project and detection-engine brand. It is always written in lowercase.

**humanymous Gate.** The reverse-proxy enforcement component that sits in front of an origin application. Use the full name on first mention and **Gate** thereafter.

**Gate.** The deployable reverse proxy. It applies route policy, enforces verdicts, meters request rates, manages bans, and records actions in a tamper-evident audit trail. The reference Gate does not reproduce every observation made by the standalone Core detection engine.

**Core detection engine.** The standalone detector used for development and reference measurement. It serves the full browser bundle and directly captures network-protocol evidence when it terminates the client connection. Measurements made against Core are not measurements of Gate.

**Ledger.** The operator console served from Gate's separate administrative listener. It shows verdicts, audit integrity, sessions, bans, policy, runtime settings, compliance actions, and pending approvals.

**Detection Observatory.** A local-only developer view served by Core. It explains how Core combined evidence and evaluated enforcement rules. It is not a production console and does not enforce traffic.

**humanymous Pass.** A keyboard-operable interactive verification exercise implemented on Core's research surface. The current reference Gate does not automatically route challenged visitors to Pass; a production integration must not claim that path until it is wired and tested end to end.

**origin application.** The application behind Gate that receives requests Gate allows. The `-upstream` flag contains the origin address; “upstream” is the flag's transport-oriented alias, not a second product component.

**public edge.** The listener that receives application traffic and applies route policy before forwarding a request to the origin.

**control plane.** The reserved paths used for browser evidence collection, trust proofs, and challenge-related exchanges. These paths are separate from the origin application's routes.

**administrative plane.** The separately bound, authenticated listener used by the Ledger, administrative application programming interface, health checks, and metrics. Production deployments must protect it with network isolation and an operator authentication layer such as mutual Transport Layer Security or single sign-on.

**reference implementation.** Code intended to demonstrate mechanisms and their boundaries. It is not a claim of production readiness, availability, capacity, or regulatory compliance.

**production responsibility.** A capability or safeguard that the reference build does not provide and that an operator must supply or harden before serving real traffic. Use this phrase instead of internal maturity shorthand.

## Verdicts and enforcement

**verdict.** The outcome attached to a scored request: ALLOW, CHALLENGE, or DENY. An edge request may also be in an unknown state while evidence is unavailable.

**ALLOW.** The request may proceed under the active route policy. On an ordinary route, Gate forwards it to the origin. A high-value attested route may still require possession or step-up proof before forwarding.

**CHALLENGE.** The request requires an additional verification step before it can proceed. A challenge is not the same as a denial. The current reference Gate does not provide a complete, user-solvable challenge path for every challenged request.

**DENY.** Gate refuses the request before contacting the origin.

**unknown state.** Gate does not yet have enough evidence to assign a verdict. Route policy and request method determine whether Gate fails open or fails closed.

**risk score.** A deterministic number from zero to one hundred that combines the evidence available to the scoring engine. Higher values mean more evidence associated with automation; the number is not proof that a person is or is not present.

**verdict threshold.** A configurable boundary that maps a risk score to CHALLENGE or DENY. The built-in defaults challenge at 30 and deny at 70. Runtime settings can change those boundaries, so documentation must label 30 and 70 as defaults rather than universal constants.

**score-based verdict.** A verdict produced by comparing the risk score with the active thresholds.

**rule-promoted verdict.** A verdict raised by a high-confidence or heuristic enforcement rule independently of the numeric score.

**enforcement rule.** A named predicate that can promote a score-based verdict or apply an edge action. Reader-facing text describes the predicate in plain language. Machine output may retain a stable technical identifier for compatibility.

**heuristic rule.** An enforcement rule that can also match some legitimate traffic. A rise in heuristic matches requires false-positive investigation before it is treated as automated traffic.

**signal.** One observation supplied to scoring, such as a browser property, an integrity result, an interaction pattern, or a network characteristic.

**top contributor.** A signal whose weighted contribution materially affected the risk score. The Ledger should display a plain-language name by default and reserve the raw signal identifier for technical details or export.

**monitor mode.** Gate scores and records what would happen but does not enforce the affected decision.

**shadow result.** A recorded “would have enforced” outcome produced while the relevant module or rule is monitoring. It supports rollout analysis without changing the request.

**enforcement mode.** Gate applies the active verdict or edge protection to the request.

**off.** A component-specific state in which a non-essential function does not run. Security-relevant enforcement rules and modules may refuse this state and allow monitoring as the weakest setting.

**fail open.** Gate lets a request proceed when required evidence or a supporting dependency is unavailable. Use only where the active route policy explicitly accepts that availability trade-off.

**fail closed.** Gate challenges or refuses a request when required evidence or a supporting dependency is unavailable. State-changing and strict routes use this posture.

**kill switch.** A dual-controlled emergency action that demotes enforcement to monitoring. Manual bans continue to apply. In the current reference build the control is node-local, not fleet-wide.

## Detection evidence

**seven-stage detection pipeline.** The complete Core pipeline: static client inspection, client fingerprinting, client integrity checks, interaction analysis, network and protocol inspection, consistency checks, and risk aggregation with verdict selection.

**static client inspection.** Observation of browser properties that commonly expose direct automation, such as WebDriver state or a headless user-agent token.

**client fingerprinting.** Derivation of a pseudonymous client characteristic from rendering, audio, screen, hardware, and related browser observations. A fingerprint is used for token binding, request metering, and cross-session comparison; it is not a person's identity.

**client integrity checks.** Checks for modified native functions, runtime hooks, prototype changes, or other evidence that browser APIs were altered.

**interaction analysis.** Observation of pointer, keyboard, scroll, and event characteristics. Accessibility policy forbids requiring motor richness or speed as proof of humanity.

**behavioral model.** An optional, policy-specific model the Core engine can load to score the aggregate features from interaction analysis. It emits one signal (`l4.ml.behavioral`) that is **weight-0 / audit-only** — it annotates the record but is never a cause of a verdict. See [How the self-correcting behavioral model works](../explanation/self-correcting-behavioral-model.md).

**behavioral model artifact.** The signed file the behavioral model is loaded from (the Core `-ml-bundle` flag). It is a different object from the **bundle** below; reserve the bare word "bundle" for the client-side loader.

**bundle (detection bundle).** The small client-side detection loader Gate injects into a page's HTML (the loader plus `detector.wasm`). Unrelated to the behavioral model artifact despite the `-ml-bundle` flag name.

**self-calibration.** The behavioral model's automatic adjustment of its weight-0 fire threshold so the realized false-positive rate on oracle-confirmed humans tracks an operator budget. It moves an annotation threshold, never a verdict.

**shadow candidate.** A second behavioral model artifact scored in parallel with the live one to preview a promotion; its output is never served or enforced. Distinct from **shadow result** (a Gate monitor-mode "would have enforced" record).

**canary (behavioral model).** A probation state for a newly deployed behavioral model that autonomously disables it — reverting to the heuristics baseline — on a false-positive, drift, or poisoning breach. Only this safe rollback direction is automatic; promotion is a redeploy.

**network and protocol inspection.** Server-side observation of connection negotiation, Hypertext Transfer Protocol version 2 behavior, and header characteristics. Core can collect the full reference set when it directly terminates the connection. The current Gate does not extract the client's full connection-negotiation fingerprint.

**consistency check.** A comparison between claims and observations, such as a browser's user agent versus the protocol implementation seen by the server. Disagreement can be stronger evidence than either value alone.

**risk aggregation and verdict selection.** The stage that combines signal contributions, applies per-stage limits, compares the result with active thresholds, and evaluates enforcement rules.

**browser fingerprint.** A derived characteristic based on browser and device observations.

**network fingerprint.** A derived characteristic based on connection negotiation and protocol behavior. A proxy or content delivery network that terminates and creates a new encrypted connection hides the original client's network fingerprint.

**user agent.** The browser or client identity claimed in a Hypertext Transfer Protocol header. It is a claim, not trusted identity.

**Transport Layer Security ClientHello fingerprint.** A characteristic derived from the cipher suites, extensions, and ordering in a client's initial encrypted-connection negotiation. JA3 and JA4 are common fingerprint formats. They are names, not guarantees of client identity.

**Hypertext Transfer Protocol version 2 fingerprint.** A characteristic derived from settings frames, header-table behavior, pseudo-header ordering, and related protocol choices.

**Chrome DevTools Protocol.** A browser-control protocol used by development tools and automation frameworks. Reader-facing text spells out the name on first use.

**WebAssembly.** A binary instruction format used by Core's browser detector. It does not make client-side logic tamper-proof.

## Challenge and trust

**proof-of-work challenge.** A computational task used to impose a measurable cost on repeated requests. It demonstrates computation, not that a person is present, and it never clears a rule-promoted verdict.

**attested route.** A high-value route configured to require possession or a step-up proof before an otherwise allowed request reaches the origin. The reference preset is `attested`.

**attestation floor.** The policy on an attested route that turns an otherwise free ALLOW into a verification requirement unless the request carries acceptable possession or step-up evidence.

**possession proof.** Cryptographic evidence that the requester controls an accepted credential, token, or key.

**step-up proof.** A short-lived, server-issued proof that an additional verification step succeeded. It is bound to the current session and cannot be treated as portable identity.

**verdict trust token.** A short-lived, fingerprint-bound token recording a previous verdict. A valid token can avoid repeated scoring on ordinary routes. A forged token or one replayed from a different fingerprint is rejected.

**request-integrity token.** A rotating, short-lived token that binds a request to its session and body with keyed message authentication. It makes header or body tampering and replay detectable.

**nonce.** A value accepted for one use so that replaying the same proof can be detected.

**replay.** Reuse of a token, proof, or request outside its allowed use. Replay protection is meaningful only when the consumed state is shared across every node that accepts the proof.

**Web Authentication.** The browser standard commonly called WebAuthn. It lets an operator verify possession of a registered public-key credential.

**Privacy Pass.** A protocol for presenting unlinkable authorization tokens issued by a trusted party.

**Web Bot Auth.** A protocol for authenticated automated clients to sign requests. It distinguishes registered automation from anonymous automation; it does not make the client human.

## Audit and privacy

**audit record.** A structured record of an administrative access, verdict, enforcement action, configuration change, approval, ban, or erasure event.

**operational log.** A best-effort diagnostic record about process lifecycle, dependencies, warnings, or internal errors. Operational logs may be formatted plain text or JSON Lines. They can be lost when a bounded queue or sink is unavailable and are not the decision or forensic authority.

**formatted plain text log.** A human-readable, one-event-per-line operational format with a UTC timestamp and stable named fields.

**JSON Lines (JSONL).** A machine-readable format containing exactly one valid JavaScript Object Notation object per physical line. humanymous uses it for local operational diagnostics, not as a standard SIEM decision schema.

**tamper-evident audit log.** An ordered record set whose hash links and signatures reveal later modification, deletion, or reordering. “Tamper-evident” does not mean impossible to alter.

**hash chain.** A sequence in which each record commits to the previous record's hash.

**keyed message authentication.** A secret-key check that detects record modification by an actor who does not hold the key. The common construction used here is a hash-based message authentication code.

**Merkle tree.** A tree of hashes that commits to many records with one root hash and supports inclusion proofs.

**signed tree checkpoint.** A signed statement of the audit tree's size and root at a point in time.

**witness.** A second signature over a checkpoint. In the reference build the witness runs in the same process and uses the same sealed keystore, so it detects malfunction or partial compromise but is not an independent external witness.

**inclusion proof.** The hashes needed to verify that a record belongs to a particular Merkle-tree checkpoint.

**write-ahead log.** The on-disk append log used to retain audit records across restarts. Without an audit log directory, the reference chain exists only in memory.

**pseudonym.** A derived identifier that replaces a raw value. Pseudonymous data can still be related back to a subject when the necessary secret material and source value are available; it is not anonymous.

**linkage key.** A per-subject secret used to derive a pseudonym. Destroying it prevents the reference implementation from deriving the same link again for that subject.

**cryptographic erasure.** Destruction of a linkage key so the associated pseudonymous records can no longer be linked by that key. The audit records remain. The operation is irreversible, and the current reference has documented durability and multi-node limits.

**hold window.** The delay between approval of an erasure request and execution, during which the scheduled action can be cancelled.

**retention.** The period for which data remains available. The reference build does not physically enforce its previously documented hot, warm, and cold retention tiers; production operators must implement and verify retention for their stores.

**incident handle.** An opaque reference that lets an operator locate an audit record without exposing detection details. The Ledger can resolve an incident handle. The default reference block page does not yet provide a usable opaque handle to every blocked visitor, so production appeal guidance must not promise one unless the deployment supplies it.

## Runtime settings

**runtime settings.** Operator-managed changes applied without restarting Gate. They can affect enforcement-rule modes, edge-protection module modes, score thresholds, selected signal weights, network evidence policy, route overrides, and request-rate limits.

**settings overlay.** The signed, versioned set of runtime differences applied on top of built-in and startup configuration.

**built-in defaults.** The behavior compiled into the reference implementation before startup configuration or runtime settings are applied.

**empty overlay.** The state in which no runtime differences are active. It preserves the built-in and startup behavior and is the authority for default regression measurements.

**effective configuration.** The resolved result of built-in defaults, startup configuration, the active settings overlay, and the kill-switch state.

**effective configuration version.** A keyed digest of the resolved policy, exposed as `config_version` in the administrative application programming interface and stamped on audit records.

**hot application.** Applying validated runtime settings to new requests without restarting Gate. A request already being scored retains the configuration version with which it started.

**proposed change.** A validated settings overlay awaiting immediate application or a separate approval, depending on whether it strengthens or weakens protection.

**protection-increasing change.** A change that makes enforcement stricter. An Operator can apply this class of change without a second approver.

**protection-reducing change.** A change that weakens a non-critical safeguard. It requires a distinct Approver and a bounded expiry.

**integrity-affecting change.** A change that weakens an integrity-critical safeguard or broadly neutralizes detection. It requires a distinct Approver, explicit typed confirmation, and a short expiry.

**sensitive-route protection reduction.** A change that weakens an attested or enforcing route. It requires a distinct Approver and a bounded expiry.

**network evidence policy.** The runtime policy that decides whether residual evidence about proxy hops, anonymity services, address spoofing, virtual private networks, Tor, and cross-session correlation can promote a verdict. Transmission Control Protocol observations remain monitor-only.

**module mode.** The monitoring or enforcement state of an edge protection such as request-smuggling rejection, trusted-header protection, bans, trust-token validation, sweep detection, or response injection.

**dry run.** An aggregate comparison of current and proposed settings that does not expose per-session data and does not change enforcement.

**rollback.** Replacement of the active settings overlay with built-in and startup behavior. Rolling back a protection-increasing change can itself weaken protection and therefore require separate approval.

**compare-and-swap.** A stale-change safeguard: a proposal applies only when its parent configuration version still matches the active version.

**rate limiter.** A component that meters requests by source characteristics over a rolling window and can trigger an escalating ban.

## Operations and governance

**Auditor.** A read-only administrative role.

**Operator.** A role that reads operational data and requests or performs permitted changes.

**Approver.** A distinct role that commits actions requiring dual-control.

**Data Protection Officer.** The oversight role used for privacy-sensitive approvals in the reference role model.

**role-based access control.** Authorization based on the authenticated operator's assigned role.

**dual-control.** A requirement that a second, distinct principal approve an action. The server derives both principals from authentication; a form field cannot nominate the approver.

**separation of duties.** Assignment of requesting, approving, and oversight capabilities to different roles.

**approval queue.** Pending dual-controlled actions awaiting a distinct authorized principal.

**automatic ban.** A temporary or escalating ban created by request-rate enforcement.

**manual ban.** A ban requested by an Operator. Permanent and network-range placement requires separate approval; lifting a ban is a single-Operator action in the current reference.

**fingerprint ban.** A ban keyed to a derived client fingerprint so it can survive a source-address change.

**network-range ban.** A ban covering an Internet Protocol address range written in Classless Inter-Domain Routing notation.

**ban ladder.** Escalating durations applied after repeated rate-limit breaches.

## Deployment and protocols

**raw Transport Layer Security termination.** Direct receipt and termination of the client's encrypted connection, with no intermediary creating a replacement connection first. It is required to observe the client's original connection-negotiation fingerprint.

**encrypted-connection re-termination.** A content delivery network, reverse proxy, or application-layer load balancer ends the client connection and opens a new one. The next hop sees the intermediary's protocol fingerprint, not the client's.

**transport-layer pass-through.** Forwarding the original Transmission Control Protocol connection without terminating its encryption.

**trusted proxy.** An explicitly configured intermediary whose source-address metadata Gate is allowed to accept. Broad trust ranges let clients forge their source and are unsafe.

**Proxy Protocol.** A transport header sent by a trusted load balancer to convey the original connection address.

**origin cloaking.** Requiring authenticated proxy-to-origin traffic so clients cannot bypass Gate and contact the origin directly.

**content delivery network.** A distributed intermediary that commonly terminates and recreates encrypted connections.

**web application firewall.** A request-inspection control focused on application attack patterns. It is complementary to automation detection.

**mutual Transport Layer Security.** An encrypted connection in which both sides present certificates.

**Automated Certificate Management Environment.** The protocol used to request and renew certificates. Its Transport Layer Security Application-Layer Protocol Negotiation challenge requires direct control of the public connection.

**Content Security Policy.** A browser security policy controlling which resources a page may load or execute.

**Server-Sent Events.** A one-way Hypertext Transfer Protocol stream used by the local Detection Observatory.

**WebSocket.** A long-lived, bidirectional connection upgraded from Hypertext Transfer Protocol.

**health probe.** A check that the process is running.

**readiness probe.** A check that the process is ready to receive traffic.

**security information and event management system.** An external system that collects, searches, and alerts on security events.

## Validation and limits

**defensive self-validation.** Running the bundled profiles only against a deployment you own or are authorized to test.

**validation profile.** A reproducible client behavior used to exercise one or more controls.

**external input control ladder.** Four sequential browser-control modes used
for defensive self-validation: virtual input; read-only Document Object Model
observation plus virtual input; physical Universal Serial Bus input; and
read-only Document Object Model observation plus physical Universal Serial Bus
input. The reference can run the first two modes in Docker. It declares the
physical modes but reports them unavailable until independently attested
hardware is supplied.

**virtual input control.** Pointer and keyboard input delivered through a
display server's software input path. It does not prove a physical device or a
person is present.

**read-only Document Object Model observation.** Browser document inspection
that can locate or describe page elements but cannot type, click, execute a
page action, or alter the document.

**Universal Serial Bus (USB).** A hardware and protocol family used to attach
devices. Public results distinguish physical USB evidence from a USB path
created by the Linux kernel.

**Human Interface Device (HID).** The USB device class commonly used for
keyboards and pointers. HID keyboard reports carry key positions and
press/release state, not Unicode text.

**Virtual USB laboratory.** The isolated workflow in which an outer Docker
Compose project manages a pinned QEMU micro virtual machine and an inner Compose
project asks that guest's independent Linux kernel to create a Universal Serial
Bus gadget through its virtual host-controller driver. It then drives a stock
browser through the guest's normal Human Interface Device input path. The input
actor cannot author the visual verdict: a separate read-only framebuffer
observer records the ordered visual evidence. The result is virtual evidence,
not physical USB or human-input evidence.

**independent framebuffer observation.** A separate, read-only process records
visible screen states without receiving a pointer, keyboard, browser-document,
clipboard, or input-method control channel. Separating observation from the
input actor prevents the actor from authoring its own visual success claim.

**kernel-emulated USB.** A USB device and host-controller path created by the
Linux kernel without an electrical cable or external device. It can exercise
enumeration, descriptors, drivers, and input routing, but cannot attest physical
transport, independent firmware, or external hardware.

**physical USB evidence.** Evidence bound to an independently attested external
device, firmware identity, physical USB path, exclusive input seat, and safety
controls. Kernel-emulated USB and a mapped Docker device are not physical USB
evidence.

**input-method composition.** The native browser and operating-system process
that turns a sequence of keyboard positions into a preedit value and then a
committed Korean, Chinese, Japanese, or other composed string. USB HID carries
keyboard usages, not Unicode text.

**Intelligent Input Bus (IBus).** The input-method framework pinned inside the
reference browser container for Korean, Simplified Chinese, and Japanese
composition checks. The laboratory gives it a private session bus and
non-persistent runtime, configuration, and cache directories; this is
laboratory isolation, not a general production configuration.

**preedit.** Text currently being composed by an input method before the user
commits it. A preedit cue alone does not prove the final value.

**committed input-method value.** Text accepted from the active input method
after its composition lifecycle finishes. The Virtual USB laboratory verifies
this separately from detector scoring and does not treat it as proof of human
input.

**human baseline.** A reference browser session used to detect accidental friction or denial of legitimate traffic. One synthetic baseline cannot establish a general false-positive rate.

**false positive.** Legitimate traffic that is challenged or denied. Report challenge friction separately from outright denial.

**false negative.** Automation that the validation policy expected to challenge or deny but that received ALLOW.

**reference-measured.** Measured under a named reference setup. It is not a guarantee for Gate, another topology, another workload, or production traffic.

**direct Hypertext Transfer Protocol automation.** A script or library that does not execute a real browser.

**off-the-shelf browser automation.** A standard browser-driver setup with common automation artifacts.

**stealth-patched browser automation.** Browser automation modified to hide common artifacts while leaving inconsistencies.

**real-browser automation.** Automation controlling a genuine browser engine with fewer reliable client-side artifacts.

**coherent browser or human-assisted automation.** Automation whose browser, network, and interaction evidence remains internally consistent, including workflows that use real people. Client and network signals alone cannot reliably distinguish this class from legitimate traffic.

**coherent-automation detection boundary.** The explicit limit described above. humanymous raises cost with request metering, reputation, and attested-route verification; it does not claim to solve this boundary.

**challenge friction.** Legitimate traffic required to complete an additional verification step. It is a user impact and must be measured separately from denial.

## Regulatory terms

**General Data Protection Regulation.** European Union data-protection law. Gate provides technical controls that may support obligations; deploying it does not make an organization compliant.

**Personal Information Protection Act.** South Korean data-protection law. As with other regulation, compliance depends on the operator's complete processing and governance.

**Data Protection Impact Assessment.** A structured assessment of personal-data processing risks and mitigations.

**data subject request.** A request by a person to exercise a data-protection right. Gate's subject boundary is session-scoped, not person-wide, and production procedures must state that limit.

## Usage rule

Use the full term from this glossary in reader-facing prose and interfaces. A standardized acronym may follow the full term only when readers must recognize that external standard or exact protocol name. Project-internal coordinate labels remain in internal specifications, plans, developer comments, tests, and compatibility fields—not in headings, navigation, explanatory prose, default badges, tooltips, command help, or ordinary logs.
