---
description: "Operator runbook for humanymous Gate's kill switch and bans. The kill switch demotes rule enforcement node-local to monitor; manual bans keep enforcing."
keywords: ["kill switch runbook","dual-control bans","humanymous Gate","fingerprint vs IP ban","escalating ban ladder","rule enforcement to monitor","on-call operator runbook","residential proxy rotation ban","self-hosted bot detection"]
---

**Diátaxis quadrant:** Runbook.
**Audience:** on-call Operators and the distinct Approver they page during an incident.

# Kill switch & bans: blast radius, apply / escalate / lift, dual-control

This runbook is written against the reference implementation, not a production-hardened build. Endpoints, defaults, and role gates match the reference; a production deployment may add controls but must not remove the dual-control gates described here.

Every action below is driven from the **Ledger** or its admin API on the separate admin listener:

- Console: `https://localhost:8445/__hmn/admin/console` (not on the public edge).
- API base: `/__hmn/admin/` on the admin listener. Bearer auth on every call. A missing or invalid token returns `404`, not `401` — that is deny-by-default, not a bug.
- In dev the console carries the **Operator** token, so it can read and request. Any two-person action must be committed by a **distinct Approver** token (a different identity from the requester). Actor identity is derived server-side from the token; request-body actor fields are ignored, and every authenticated access is meta-audited before it is served.

The dual-control commit flow is always the same three steps:

1. **Request** the action (Operator) — it lands as `PENDING`.
2. **Locate** the pending id: `GET /__hmn/admin/approvals`.
3. **Commit** it as a **distinct Approver**: `POST /__hmn/admin/approvals/<id>`.

> **Note:** Request/response shapes — `POST /bans` takes `{"Key","Reason","Incident","DurationSec"}`; `/bans/bulk` takes `{"Keys":[…],"Reason","DurationSec"}` (temporary only); `/bans/lift` takes a **`?key=<ban-key>` query parameter, not a body**; `/killswitch` takes `{"On":true}`. Full shapes are in the [CLI, config & policy reference](../reference/cli-config-policy.md#request--response-shapes).

---

## Part 1 — The kill switch

### What it does, exactly

The kill switch demotes **enforcement rules to monitor on the current Gate node**. Detection does not stop: Gate continues to score its available evidence and write decisions to the audit log, but rule-promoted CHALLENGE or DENY no longer takes effect. A multi-node deployment must coordinate this state externally before calling it node-local.

One thing keeps enforcing: **manual bans still enforce under the kill switch.** A ban you placed on an `ip:`, `fp:`, or `cidr:` key continues to block while the switch is on. The kill switch demotes the scoring-and-enforcement-rule path, not your explicit denylist.

> **Warning:** The kill switch is **node-local and not route-scoped**. It demotes rule enforcement for every route on the current node. A fleet must coordinate the action externally and confirm every node separately.

### Why it is dual-control

A node-local enforcement stand-down is too consequential for one person, so it is gated on two distinct roles:

1. **Operator requests** the flip: `POST /__hmn/admin/killswitch` — it lands as `PENDING`.
2. **A distinct Approver commits** it: `GET /__hmn/admin/approvals` → `POST /__hmn/admin/approvals/<id>`.

The requester cannot approve their own flip. If you are the Operator, you cannot proceed without paging an Approver; page them before you request, not after.

Both directions — engaging the switch and disengaging it — cross the same two-person gate:

```mermaid
stateDiagram-v2
  [*] --> Enforcing
  Enforcing --> PendingOn: Operator requests kill switch ON
  PendingOn --> KillSwitchOn: distinct Approver commits
  PendingOn --> Enforcing: not committed
  KillSwitchOn --> PendingOff: Operator requests kill switch OFF
  PendingOff --> Enforcing: distinct Approver commits
  note right of KillSwitchOn: enforcement rules demoted to monitor node-local; manual bans still enforce
```


### The decision criterion — when to pull it, when not to

Pull the kill switch for **a false-positive storm that is locking out real customers faster than you can tune it.** That is the one situation where letting automation through briefly is the lesser harm: real humans are being denied, and the switch stops the bleeding node-local while you fix the misfiring rule or route.

Do **not** pull the kill switch to "calm" an actual attack. Under a real scraping, flood, or AI-agent wave, the enforcement rules are the thing protecting the origin — demoting them node-local hands the attacker exactly the access they wanted. For attacks, use bans (Part 2); keep enforcement on.

If you are unsure whether a DENY spike is a misfire or an attack, run the attack-or-false-positive triage in the [Incident runbooks](./incident-runbooks.md) before you touch the switch.

### The narrower alternative, and why it is not always available

The least-disruptive fix for a misfiring route is to demote **that one route** to monitor, leaving enforcement intact everywhere else. But per-route demotion is **not a runtime option**: route presets (`off` / `monitor` / `balanced` / `strict` / `attested`) are startup configuration in the route table, not something an admin endpoint rewrites live. `GET /__hmn/admin/policy` reads the posture; there is no runtime policy-write endpoint.

So under a false-positive storm your two runtime choices are:

- **Edit the route table and restart** — demotes the single misfiring route to `monitor`, narrow blast radius, but costs a restart.
- **The global kill switch** — no restart, immediate, but node-local.

Pick the narrow route-table edit when you can afford the restart and the misfire is confined to one route. Pick the kill switch when the misfire is broad or a restart is too slow while customers are locked out.

### Consequence and rollback in one breath

- **Consequence:** rule enforcement is off for every route on the current node. Detection still scores and logs; manual bans still enforce.
- **Rollback:** flip the kill switch back off. This is **also dual-control** and needs a **distinct Approver** — request as Operator, commit as Approver. Do it the moment the misfiring rule is tuned or the route is demoted and restarted.

---

## Part 2 — Bans

Bans are your enforcement lever for an actual attack, and they escalate on their own. Auto-bans climb an escalating ladder on repeat strikes:

**1h → 6h → 24h → permanent**, with strike decay so a source that stops misbehaving falls back down over time. Let the ladder do the work; you place manual bans only to get ahead of it.

`Source=auto` marks a ban the ladder placed; `Source=manual` marks one you placed. The active set is visible in `GET /__hmn/admin/bans`, and the "Rate Limits & Bans" console view shows the count as a badge.

### `ip:` vs `fp:` — pick the stable handle

A ban keys on either an IP or a fingerprint, and choosing wrong either misses the target or catches humans:

- **`fp:<fingerprint>` — use this for residential-proxy rotation (cross-session correlation rule).** When one stable fingerprint appears across many subnets, the IP is a moving target and the fingerprint is the stable handle. An `ip:` ban here chases the rotation and, worse, an over-broad IP or CIDR ban risks blocking real humans behind CGNAT or a corporate egress that shares those addresses. Ban the fingerprint.
- **`ip:<addr>` or `cidr:<range>` — use this for a fixed hostile source.** When many fingerprints come from one hostile IP or range, the IP is the stable handle and a fingerprint ban would chase each new fingerprint.
- **Neither beats shared-egress false positives.** Many real humans behind one NAT or carrier IP means: do not IP-ban. Prefer fingerprint-scoped action or let rate metering absorb it.

### Applying bans

- **Single temporary ban** (`1h` / `6h` / `24h`, `ip:` or `fp:`): `POST /__hmn/admin/bans`. **Single Operator action** — reversible when it expires.
- **Bulk temporary bans:** `POST /__hmn/admin/bans/bulk`. **Temporary-only by design** — permanent and CIDR keys are **rejected** from bulk. Use bulk to place a batch of expiring `fp:`/`ip:` bans quickly; it cannot be used to place indefinite or range bans.

> **Warning:** A **permanent** ban or a **CIDR** ban is **dual-control**. Request it as Operator, then have a **distinct Approver** commit via `POST /__hmn/admin/approvals/<id>`.
> - **Consequence:** the key is denied indefinitely (permanent) or an entire range is denied (CIDR) — high blast radius, real risk of catching humans behind shared egress.
> - **Rollback:** `POST /__hmn/admin/bans/lift` — **single Operator action** for every ban class (temporary, permanent, and CIDR). Placement of permanent/CIDR is dual-control; lift is not.

### Lifting bans — the rollback path

- **Every lift is a single Operator action:** `POST /__hmn/admin/bans/lift?key=<ban-key>`. The reference routes lift to `adminCanOperate` only — there is no Approver commit path for unblocks. Permanent/CIDR **placement** is dual-control; **removal** is not.

### What bans are *not* for

Do not reach for the kill switch to deal with an attack that bans can hold. The kill switch demotes rule enforcement node-local and would remove exactly the DENY that is protecting the origin — while your manual bans keep enforcing regardless. Escalate the ladder with fingerprint bans; keep enforcement on.

> **Note:** Anti-detect tooling combined with real-human click-farms is an explicit design boundary Gate does not solve; it is mitigated only by rate and reputation. If bans are not denting the volume, escalate rather than banning ever-wider.

---

## Quick reference

| Control | Who requests | Who commits | Blast radius | Rollback |
| --- | --- | --- | --- | --- |
| Kill switch (on or off) | Operator | **Distinct Approver** | Node-local; rule enforcement → monitor everywhere; manual bans still enforce | Flip off — also dual-control |
| Temporary `ip:`/`fp:` ban | Operator (single) | — | One key, expires (1h/6h/24h) | Lift — single Operator |
| Bulk temporary bans | Operator (single) | — | Batch of expiring keys (perm/CIDR rejected) | Lift each — single Operator |
| Permanent or CIDR ban | Operator | **Distinct Approver** | Indefinite key, or whole range | Lift — single Operator |

### Rules of engagement

- **Kill switch and permanent/CIDR ban placement are dual-control** — always name and page a **distinct Approver**; the requester never self-approves. **Ban lifts are single-Operator** (any ban class).
- **The kill switch is node-local** — it demotes rule enforcement for every route on that node. Manual bans still enforce. Coordinate and verify each node in a fleet.
- **Per-route demotion is startup config, not a runtime toggle** — under a false-positive storm your runtime choices are the global kill switch or a route-table edit plus restart.
- **`fp:` beats `ip:` against rotation (cross-session correlation rule)** — and avoids CGNAT / corporate-egress false positives; `ip:`/`cidr:` beats fingerprint against a fixed source; neither beats shared-egress false positives.

### Related pages

- [Incident runbooks](./incident-runbooks.md) — symptom-indexed on-call procedures and the attack-or-false-positive triage.
- [Right-to-erasure (crypto-shred) runbook](./erasure-crypto-shred.md) — the other dual-control control plane.
- [role-based access control & separation of duties reference](../reference/rbac-separation-of-duties.md) — Auditor / Operator / Approver / data protection officer capabilities and who can commit what.
- [enforcement rules, verdicts & signal reference](../reference/hard-rules-verdicts.md) — cross-session correlation rule and the enforcement-rule glossary.
- [CLI, config & per-route policy reference](../reference/cli-config-policy.md) — route presets and the `-monitor` flag.
