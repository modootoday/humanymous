# Red-team rules of engagement (defensive-only)

**Diátaxis quadrant:** Reference.
**Audience:** defensive-test developers and evaluators; read this before running or extending bundled adversarial test tooling.

> **Defensive, local-only, self-target-only, educational.** The bundled Red tooling exercises **your own** detector's test catalog on **your own** loopback interface. It teaches you to understand and extend the detection engine you run — never to evade a third party. Point every tool at a host you operate; keep it on `127.0.0.1`. If a check reveals a coverage gap, report it to the maintainers — never optimize it into an evasion.

This is the one page every advanced Red document links to for its safety framing, so that framing cannot drift. It states the scope, the boundary, the difference between a **structural** loopback lock and an **advisory** default, the honest ceilings to repeat when you report, and the good-faith safe-harbor pointer with its hard limit. It ends with a copy-paste **guardrail preamble block** that the advanced Red how-to pages embed verbatim at the top.

## Scope

This tooling exercises **your own detector's test catalog on your own loopback** — the standalone detection engine on `127.0.0.1:8443`. It is a way to confirm that your detector fires the verdicts and enforcement rules you expect against known-automated samples, and to measure friction on the fixed synthetic human baseline before you enforce anything.

It is **not** evasion guidance against any third party. Nothing here tells you how to defeat someone else's deployment, and the Red profiles reproduce *tells* an automated client leaves for your engine to catch — they are not a toolkit aimed outward. For what the catalog is and how it is wired, see **[Red-team catalog](red-team-catalog.md)** and **[Red catalog architecture](../explanation/red-catalog-architecture.md)**.

## The three-line boundary

Repeat these three lines on every advanced Red page and hold to them:

1. **Target only hosts you operate.** The tooling runs against the standalone detection engine on your own loopback (`127.0.0.1:8443`).
2. **Never point any tool at a domain, IP, or deployment you do not run.** No exceptions for "just testing", "it's only one request", or "I have permission verbally". Written authorization for a system you do not operate is required, and even then this tooling is not the vehicle for it (see the safe-harbor limit below).
3. **Findings about coverage go to the maintainers — never optimized into an evasion.** A gap you discover in your own detector is a maintenance signal for the detection engine, not a recipe to publish or aim at anyone.

## Structural locks vs advisory defaults

Not every safety property is enforced the same way. Know which is which before you run anything, because the difference decides how much of the responsibility is yours.

### Structurally loopback-locked — the Detection Observatory launcher

The Observatory launcher is locked by construction; the target cannot be redirected off your box:

- The launch target is **hard-coded to `127.0.0.1:8443`** — there is no target field to set.
- Any `host`, `url`, or `upstream` key in a launch request is rejected with **HTTP 400**.
- Each launch requires a **single-use nonce**.
- A **cross-origin `Origin`** on launch is refused.
- `run-one.mjs` performs its **own loopback check** and rejects a non-loopback base and path-traversal profile names.
- A **per-request `Host` loopback check** re-validates on every request (a DNS-rebinding defense beyond the bind-time check), and startup is **fail-closed** — the engine refuses a non-loopback `-addr`.

These are enforcement points, not conventions. See **[Detection Observatory](../how-to/detection-observatory.md)** and **[Observatory architecture](../explanation/observatory-architecture.md)**.

### Advisory only — `cmd/redteam -host`

One tool is different. The Go attack CLI `cmd/redteam` takes a `-host` flag that **defaults to `127.0.0.1:8443`**, but that default **enforces nothing** — the CLI connects to whatever `host:port` you pass it.

> **Warning:** On `cmd/redteam` there is no structural lock. Keeping the target on your own box is the **operator's responsibility**, not the tool's. Do not pass a `-host` you do not operate. (The binary's own `-attack` flag help is currently stale; the real attack values are documented in **[Red-team catalog](red-team-catalog.md)**.)

## Honest ceilings — repeat these when you report

Detection is bounded and heuristic. When you report a result, carry these limits with the number so no reader mistakes a measurement for a guarantee:

- **The coherent browser or human-assisted automation boundary is not solved.** Anti-detect tooling combined with real-human click-farms is a design boundary that is **mitigated by rate and reputation, not solved**. A pass or a block against a coherent browser or human-assisted automation-style sample does not close that gap.
- **datacenter-network browser rule, no interaction observed rule, and missing or replayed request-integrity token rule are heuristics that can challenge real humans.** datacenter-network browser rule (a consistent browser from a datacenter ASN), no interaction observed rule (zero interaction over the observation window), and missing or replayed request-integrity token rule (a replayed or absent request-integrity token on an API call) issue **CHALLENGE**, not DENY, and can catch some genuine people. Do not report them as clean bot-only signals.
- **Every number is reference-measured on your machine.** A detection rate or false-positive rate describes **that run on your box** — it is not a promise about live traffic. Inspect the fixed synthetic human baseline's challenge rate separately; a human CHALLENGE is scored as a true negative, so a DENY-only false-positive rate under-reports human friction.

For the verdict thresholds and the Core enforcement-rule table, see **[enforcement rules, verdicts and signal reference](hard-rules-verdicts.md)**.

## Good-faith safe harbor — and its hard limit

Testing conducted in good faith **against systems you operate, or a deployment you are explicitly authorized in writing to test**, is welcomed under the coordinated-disclosure policy.

> **Warning:** The safe harbor is **not** authorization to test anything you neither operate nor are authorized in writing to test. This is a defensive product; its Red-team and self-validation tooling targets **your own** deployment only. Testing an instance you do not run and do not have written permission to test is outside the policy, is not authorized, and may be unlawful.

Read the full policy — scope, how to report, out-of-scope items, and the safe-harbor wording — in **[Security vulnerability disclosure policy](security-disclosure.md)**. To actually run the self-validation exercise on your own install, follow **[Self-validation: red-team your own Gate deployment](../how-to/self-validation-red-team.md)**.

## Guardrail preamble block (embed verbatim)

Every advanced Red how-to page opens with the block below. Copy it verbatim to the top of the page, immediately after the title and the Diátaxis/audience line, so the framing stays identical across the docs set.

```
> **Defensive, local-only, self-target-only, educational.** This page helps you
> understand and extend **your own** detector's test catalog on **your own**
> loopback (`127.0.0.1:8443`). It is not evasion guidance against any third
> party. Target only hosts you operate; never point any tool at a domain, IP, or
> deployment you do not run; report coverage gaps to the maintainers rather than
> optimizing them into an evasion. Honest ceilings apply: coherent browser or
> human-assisted automation is rate/reputation-mitigated, not solved;
> datacenter-network, no-interaction, and missing-request-token heuristics can challenge real
> humans; every number is reference-measured on your machine, not a guarantee.
> The Observatory launcher is structurally loopback-locked, but `cmd/redteam
> -host` enforces nothing — keeping that target on your own box is your
> responsibility. See the full
> [Red-team rules of engagement](../reference/red-team-rules-of-engagement.md)
> and the [security disclosure policy](../reference/security-disclosure.md).
```

> **Note:** The relative links inside the preamble (`../reference/…`) are written for a page in `docs/how-to/` or `docs/explanation/`. Adjust the path depth if you embed the block at a different directory level.

## Related

- **[Red-team catalog](red-team-catalog.md)** — the profiles, their expected enforcement rules, and the real `cmd/redteam` attack values.
- **[Red catalog architecture](../explanation/red-catalog-architecture.md)** — how the profiles, runner, and CLI fit together.
- **[Write a Red profile](../how-to/write-a-red-profile.md)** — the profile contract and the three registration points.
- **[Run the detection engine](../how-to/run-detection-engine.md)** — build and start the standalone engine on loopback.
- **[enforcement rules, verdicts and signal reference](hard-rules-verdicts.md)** — verdict thresholds and the Core rules table.
- **[Security vulnerability disclosure policy](security-disclosure.md)** · **[Self-validation and red-team testing](../how-to/self-validation-red-team.md)**.
