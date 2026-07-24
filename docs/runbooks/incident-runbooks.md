---
description: "Symptom-indexed on-call runbooks for a humanymous Gate deployment: triage and respond to DENY-spike false positives, scraping, HTTP/2 floods, AI-agent waves and audit-integrity alerts, with dual-control rollbacks."
keywords: ["humanymous Gate incident runbooks","on-call SOC runbook","false-positive storm triage","credential-stuffing response","HTTP/2 flood mitigation","AI-agent wave","audit integrity mismatch","dual-control kill switch","fingerprint vs IP ban","reverse-proxy bot detection"]
---

> **Runbook** — operational procedures for SOC on-call responders.
> **Audience:** on-call SOC / incident responders operating a running humanymous Gate deployment. Not an intro; if you have never seen the Ledger, read [How Gate sees a request](../concepts/how-gate-sees-a-request.md) first.

# Incident runbooks: humanymous Gate on-call

This page is symptom-indexed. Find the symptom you are seeing, run the 60-second triage, follow the decision tree, take the exact action, and know your rollback before you commit it.

The triage fork every section shares — is this a false positive to relieve, or an attack to hold?

```mermaid
flowchart TD
  S["Symptom on the Overview view"] --> Q{"60-second triage: attack or false-positive?"}
  Q -->|"humans locked out, one route or one rule"| FP["False-positive storm (a)"]
  Q -->|"automation signals across subnets"| AT["Real attack"]
  FP --> D1["Least-disruptive: demote misfiring route to monitor"]
  FP --> KS["Broad lockout only: kill switch — fleet-wide, dual-control"]
  AT --> B["(b) Scraping / cred-stuffing — temporary fp: ban"]
  AT --> H["(c) HTTP/2 flood — confirm auto-DENY plus limiter"]
  AT --> AG["(d) AI-agent wave — temporary fp: ban"]
  S --> INT["(e) Integrity red — verify offline, preserve evidence, do NOT write"]
```



humanymous Gate is a reverse-proxy enforcement layer that scores every request across layers L1–L7, applies hard rules, and enforces a verdict (ALLOW / CHALLENGE / DENY) at the edge. This is a reference implementation, not a production-hardened build; your deployment's paging rotation and thresholds may differ.

## Before you start: the tools you will use

You drive every action below from the **Ledger** or its admin API.

- Console: `https://localhost:8445/__hmn/admin/console` (served on the separate admin listener, not the public edge).
- API base: `/__hmn/admin/` on the admin listener. Bearer auth on every call. A missing or invalid token returns `404`, not `401` — that is deny-by-default, not a bug.
- In dev the console carries the **Operator** token, so it can read and request actions. Any two-person action must be committed by a **distinct Approver** token — the requester cannot approve their own action.

Read-only triage endpoints you will lean on:

```
curl -sk "https://localhost:8445/__hmn/admin/audit?verdict=deny" -H "Authorization: Bearer <operator-token>"
```

The `/audit` endpoint accepts `?verdict=&host=&route=&rule=&minRisk=&before=` and returns a `nextBefore` cursor for paging. Other reads: `/bans`, `/integrity`, `/incidents/<handle>`, `/policy`, `/approvals`, `/whoami`.

> **Note:** Actor identity is derived server-side from your token. You cannot act as someone else by editing a request body; those fields are ignored. Every authenticated admin access is itself meta-audited before it is served.

### The dual-control actions (memorize these)

Three action classes require a **distinct Approver** (second person, different token from the requester):

1. **Kill switch** (fleet-wide).
2. **Permanent or CIDR bans**.
3. **Ban lift / unblock** of a dual-control ban, and **erasure**.

The flow is always the same:

1. **Request** the action as Operator — it lands as `PENDING`.
2. **Locate** the pending id: `GET /__hmn/admin/approvals`.
3. **Commit** it as a distinct Approver: `POST /__hmn/admin/approvals/<id>`.

> **Note:** Request/response shapes — `POST /bans` takes `{"Key","Reason","Incident","DurationSec"}`; `/bans/bulk` takes `{"Keys":[…],"Reason","DurationSec"}` (temporary only); `/bans/lift` takes a **`?key=<ban-key>` query parameter, not a body**; `/killswitch` takes `{"On":true}`. Full shapes: [CLI, config & policy reference](../reference/cli-config-policy.md#request--response-shapes).

---

## (a) DENY rate jumped and real users are locked out

Suspected **false-positive storm**: a spike in DENY verdicts that is hitting verified humans, not automation.

### Trigger signal

- The Overview view ("live edge decisions") shows DENY climbing sharply.
- Support/appeal volume rises; users report a block page with an incident handle.

### 60-second triage — attack or false-positive?

1. **Filter** the audit log to the deny spike.

```
curl -sk "https://localhost:8445/__hmn/admin/audit?verdict=deny&minRisk=70" -H "Authorization: Bearer <operator-token>"
```

2. **Read** the TopContributors on the drilled-down sessions. Real automation clusters on high-confidence signals (`l1.navigator.webdriver`, `l1.cdp.proxy_leak`, `x.ua_vs_ja4`). A false-positive storm clusters on a single route or a single heuristic rule (for example HR-12, no-interaction → CHALLENGE) firing on ordinary traffic.
3. **Check** whether the denies concentrate on one route (`?route=`) or one rule (`?rule=`). One route + one rule + diverse, plausible fingerprints = misfire. Many rules + automation signals across subnets = real attack.

### Decision tree

- **Denies are correct automation** → this is not your incident; go to (b), (c), or (d).
- **One route is misfiring** on otherwise-human traffic → demote that route to monitor (below). Least-disruptive.
- **Misfire is broad / cross-route and users are locked out fleet-wide** → consider the **kill switch**. Fleet-wide and dual-control; read the warning first.

### Action — demote the misfiring route to monitor

Demoting a route to `monitor` keeps scoring and logging but enforces nothing on that route, so blocked humans flow through while you investigate.

> **TODO(verify):** the runtime mechanism/endpoint to change a single route's policy preset to `monitor` from the admin API. The admin API in the brief exposes `GET /policy` (read) but no policy-write endpoint; the reference build may require a config/route-table change and restart (`-monitor` is global only). Confirm before relying on a hot per-route demotion.

- **Consequence:** the demoted route stops enforcing — automation on that route is scored and logged but passes to origin.
- **Rollback:** restore the route's prior preset (`balanced` or `strict`) once the misfiring rule is tuned.

### Action — kill switch (broad lockout only)

> **Warning:** The kill switch is **fleet-wide** and **dual-control**. It demotes hard-rule enforcement to monitor across every node — detection continues to score and log, but hard-rule DENY/CHALLENGE stops taking effect and traffic flows to origin. Manual bans still enforce under the kill switch. Use it only when a false-positive storm is locking out real users faster than you can tune, and you accept that automation flows freely while it is on.

1. **Request** the flip as Operator: `POST /__hmn/admin/killswitch`.
2. **Have a distinct Approver commit it**: `GET /__hmn/admin/approvals` → `POST /__hmn/admin/approvals/<id>`.

- **Consequence:** hard-rule enforcement is off fleet-wide. The origin is exposed to automation that hard rules would otherwise DENY.
- **Rollback:** flip the kill switch back off — this is **also dual-control** and needs a **distinct Approver**. Do it the moment the misfiring rule is tuned or the route is demoted.

### Who to page

- **Page the Operator on-call** to tune/demote the route.
- **Page a distinct Approver** immediately if a kill switch or a dual-control unblock is on the table — you cannot proceed without one.

---

## (b) Scraping / credential-stuffing surge

High-volume automated access, often behind rotating residential proxies.

### Trigger signal

- Sustained DENY/CHALLENGE on login/checkout routes.
- Auto-ban ladder is climbing; `HR-19` (one stable fingerprint across many subnets) or `HR-30` (many sessions from one fingerprint) appears in the audit filter.

### 60-second triage — attack or false-positive?

1. **Filter** for the rotation signature.

```
curl -sk "https://localhost:8445/__hmn/admin/audit?rule=HR-19" -H "Authorization: Bearer <operator-token>"
```

2. **Look** at the ban keys in `GET /__hmn/admin/bans`. If one `fp:<fingerprint>` recurs across many distinct IPs/subnets, this is residential-proxy rotation — the fingerprint is stable, the IP is not.
3. **Confirm** it is not a shared-egress false positive (a corporate NAT or mobile carrier can put many real humans behind few IPs). Rotation shows one fingerprint across many subnets; shared egress shows many fingerprints behind one IP.

### Decision tree

- **One fingerprint, many subnets (`HR-19`)** → ban on the **`fp:` key, not `ip:`**. An IP ban chases a moving target; the fingerprint is the stable handle.
- **Many fingerprints behind one hostile IP/range** → an `ip:` ban is appropriate; a CIDR ban is dual-control.
- **Many real humans behind one IP** → do **not** IP-ban. Prefer fingerprint-scoped action or let rate metering absorb it.

### Action — fingerprint ban with the escalating ladder

Auto-bans escalate on repeat strikes: **1h → 6h → 24h → permanent**, with strike decay for good behavior. Let the ladder do the work; you place manual bans only to get ahead of it.

- **Temporary `fp:` ban** (1h/6h/24h): `POST /__hmn/admin/bans`. Single-person Operator action; reversible when it expires.
- **Bulk temporary bans**: `POST /__hmn/admin/bans/bulk`. Temporary-only by design — permanent and CIDR keys are **rejected** from bulk.

> **Warning:** A **permanent** or **CIDR** ban is **dual-control**. Request it as Operator, then have a **distinct Approver** commit via `POST /__hmn/admin/approvals/<id>`.
> - **Consequence:** the key is denied indefinitely (permanent) or an entire range is denied (CIDR) — high blast radius, real risk of catching humans behind shared egress.
> - **Rollback:** `POST /__hmn/admin/bans/lift`. Lifting a dual-control ban is **also dual-control** — needs a **distinct Approver**.

### Who to page

- **Page the Operator on-call** to place fingerprint bans and watch the ladder.
- **Page a distinct Approver** for any permanent/CIDR ban or any lift.

---

## (c) HTTP/2 flood / destructive abuse (HR-21)

Protocol-level flooding or a destructive request pattern.

### Trigger signal

- `HR-21` (destructive flood / HTTP/2 DoS / rate-limit ban → DENY) fires.
- Signals `l5.h2dos.rapid_reset` and related appear in TopContributors.
- Control-plane flood limiter is returning `429` before scoring (default window 10s, soft 60, hard 120 per IP).

### 60-second triage — attack or false-positive?

1. **Filter** for the rule.

```
curl -sk "https://localhost:8445/__hmn/admin/audit?rule=HR-21" -H "Authorization: Bearer <operator-token>"
```

2. **Confirm** it is protocol abuse, not a load spike: `HR-21` keys on frame-level patterns (rapid reset) and rate-limit ceilings, not on content. A legitimate traffic surge raises request counts but does not produce reset-flood frame signatures.

### Decision tree

- **`HR-21` firing + rate limiter shedding at `429`** → enforcement is already active; DENY is automatic and the origin is never contacted. Your job is containment and escalation, not first-line blocking.
- **Repeat offender by fingerprint or IP** → place a ban (fingerprint for a rotating source, IP/CIDR for a fixed source) to move it up the ladder faster.
- **Rate ceilings are too low and catching real bursts** → the flood limiter thresholds (`RateWindow` / `RateSoft` / `RateHard`) are configuration, not a hot admin toggle; retuning them is a config/restart change — escalate, do not guess.

### Action

- **Ban the source** with the ladder as in (b): temporary `fp:`/`ip:` ban via `POST /__hmn/admin/bans`; permanent/CIDR is **dual-control** with a **distinct Approver**, rollback via `POST /__hmn/admin/bans/lift` (also dual-control).
- **Do not** reach for the kill switch here — it demotes hard-rule enforcement fleet-wide and would remove exactly the `HR-21` DENY that is protecting the origin.

### Who to page

- **Page the Operator on-call** for bans.
- **Page the infrastructure/network on-call** if the flood is saturating the edge below the application layer — Gate sheds at the control plane, but upstream capacity is outside its control.

> **TODO(verify):** the concrete escalation contact / rotation for HTTP/2 DoS saturation (deployment-specific; not in the brief).

---

## (d) AI-agent wave (HR-20)

A surge of AI browser-agents driving real browser engines.

### Trigger signal

- `HR-20` (AI browser-agent signature: teleport click + LLM cadence → DENY) fires across many sessions.
- Signals `l4.agent.llm_cadence` and `l4.mouse.click_no_trajectory` dominate TopContributors.

### 60-second triage — attack or false-positive?

1. **Filter** for the rule.

```
curl -sk "https://localhost:8445/__hmn/admin/audit?rule=HR-20" -H "Authorization: Bearer <operator-token>"
```

2. **Inspect** the behavioral evidence. `HR-20` keys on teleport clicks (no cursor trajectory) plus LLM-paced cadence. These are behavioral tells on real engines (attacker tier T3), so confidence is lower than for hardware artifacts — read a few drill-downs before acting broadly.
3. **Watch the false-positive edge:** assistive tech, scripted accessibility tools, and some automation-for-testing can resemble teleport-click cadence. If drill-downs look like genuine assisted humans, treat it like a false-positive storm — go to (a).

### Decision tree

- **Clear agent signature, high volume** → `HR-20` is already DENYing. Add fingerprint bans for the persistent sources to escalate the ladder.
- **Mixed / uncertain evidence** → keep enforcing but hold manual permanent bans; prefer temporary `fp:` bans that expire.
- **Evidence looks like assisted humans** → demote the affected route to monitor (see (a)) and page for policy review rather than banning.

### Action

- **Temporary `fp:` ban** via `POST /__hmn/admin/bans` for repeat agent fingerprints; the ladder escalates on repeats.
- **Permanent/CIDR** only with a **distinct Approver** (dual-control); rollback via `POST /__hmn/admin/bans/lift` (also dual-control).

> **Note:** T4 (anti-detect tooling plus real-human click-farms) is an explicit design boundary Gate does not solve; it is mitigated only by rate and reputation. If bans are not denting the volume, escalate rather than banning ever-wider.

### Who to page

- **Page the Operator on-call** for bans and route review.
- **Page a distinct Approver** for any permanent/CIDR ban.

---

## (e) Integrity Dashboard went red

The audit chain reported a mismatch. **This is not a routine bot page.** Treat it as possible tampering until proven otherwise.

### Trigger signal

The Integrity Dashboard flags one of these mismatch classes:

- **hash-break** — a record's content hash does not match.
- **hmac-invalid** — a per-record HMAC does not verify.
- **seq-gap** — a sequence number is missing.
- **linkage-break** — the append-only chain linkage is broken.
- **checkpoint-mismatch** — a Signed Tree Head checkpoint does not reconcile.
- **node-missing** — records for a node are absent (a suppression alert).

### 60-second triage — attack or false-positive?

1. **Read** the integrity state.

```
curl -sk "https://localhost:8445/__hmn/admin/integrity" -H "Authorization: Bearer <auditor-token>"
```

2. **Identify** the mismatch class and the affected node/segment.
3. **Recall** the honest scope: records written after the last signed checkpoint are re-writable by the writer until the next checkpoint (`CheckpointEvery=32`) — the unanchored in-window residual. A mismatch confined to that live tail is the known residual; a mismatch **before** the last checkpoint, or a failed witness co-sign, is a tamper indicator.

### Decision tree

- **hash-break / hmac-invalid / linkage-break / checkpoint-mismatch before the last checkpoint** → treat as tampering. Preserve evidence. Escalate.
- **node-missing (suppression)** → treat as tampering or outage of the audit writer. Escalate — silence is itself the alarm.
- **seq-gap only in the live unanchored tail** → likely the in-window residual or a writer restart; still verify with the offline verifier before standing down.

### Action — verify independently, do not mutate

> **Warning:** Do **not** place bans, lift bans, flip the kill switch, or issue erasures while integrity is red and unexplained. Those actions write new audit records and can obscure the evidence. Contain by escalation, not by admin writes.

1. **Run** the offline verifier against the audit log. It checks the hash chain, per-record HMACs, and Ed25519 Signed Tree Heads with the independent witness co-signature, **without trusting the operator or the running node**.

> **TODO(verify):** the exact offline-verifier invocation (binary/subcommand and arguments) for the reference build; the brief states an offline verifier exists but not its command line.

2. **Capture** the verifier output and the Integrity Dashboard state as evidence (screenshots + saved output).
3. **Preserve** the audit files; do not rotate, truncate, or restart the node until security has the evidence.

- **Consequence of a confirmed break:** the audit record is only tamper-evident, not tamper-proof — a confirmed pre-checkpoint break means you cannot trust unanchored records from that window. Scope the blast radius with security.
- **Rollback:** none — you do not "undo" an integrity mismatch. You confirm it, preserve it, and hand it off.

### Who to page

- **Page the security lead** immediately. An integrity mismatch is a security incident, not a traffic incident.
- **Page the DPO** as well if the affected segment touches subject data or an in-flight erasure — see the [Right-to-erasure (crypto-shred) runbook](./erasure-crypto-shred.md).

---

## Quick reference

| Symptom | Primary rule / signal | First non-destructive move | Escalation |
| --- | --- | --- | --- |
| (a) DENY spike, humans locked out | HR-12 misfire / route-scoped | Demote route to monitor | Distinct Approver for kill switch |
| (b) Scraping / cred-stuffing | HR-19, HR-30 | Temporary `fp:` ban | Distinct Approver for permanent/CIDR |
| (c) HTTP/2 flood | HR-21, `l5.h2dos.rapid_reset` | Confirm auto-DENY + limiter | Infra/network on-call |
| (d) AI-agent wave | HR-20, `l4.agent.llm_cadence` | Temporary `fp:` ban | Distinct Approver; Operator for route review |
| (e) Integrity red | mismatch classes | Run offline verifier, preserve | Security lead (+ DPO if data-touching) |

### Rules of engagement

- **Kill switch and permanent/CIDR bans and their lifts are dual-control** — always name and involve a **distinct Approver**; the requester never self-approves.
- **The kill switch is fleet-wide** — it stops hard-rule enforcement everywhere and exposes the origin; manual bans still enforce; roll it back (dual-control) the moment the cause is fixed.
- **Fingerprint (`fp:`) beats IP (`ip:`)** against rotation; IP/CIDR beats fingerprint against a fixed hostile source; neither beats shared-egress false positives.
- **Integrity red is a security incident** — verify offline, preserve evidence, do not paper over it with admin writes.

### Related pages

- [Hard rules, verdicts & signal-ID reference](../reference/hard-rules-verdicts.md)
- [CLI, config & per-route policy reference](../reference/cli-config-policy.md)
- [Right-to-erasure (crypto-shred) runbook](./erasure-crypto-shred.md)
- [Will this break my app? (safety, fail-open, rollout)](../explanation/will-this-break-my-app.md)
- [Why am I seeing this? (blocked-user help + appeal)](../help/why-am-i-seeing-this.md)
