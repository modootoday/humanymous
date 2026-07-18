---
title: Deployment & Policy Operations
---

# Deployment & policy operations

**Diátaxis quadrant:** How-to. **Audience:** operators and integrators deploying humanymous Gate ("Gate" after first mention) beyond a first local run.

This page is a shelf of independent, goal-titled recipes. Each follows the same template — a goal, numbered steps, and a way to verify — so you can jump to the one task you need and stop. Concepts and full option tables are linked, not repeated here: for the mechanism behind a request see [How Gate sees a request](../concepts/how-gate-sees-a-request.md), and for every flag, preset, and threshold see the [CLI, config & per-route policy reference](../reference/cli-config-policy.md).

> **Note:** This repository is a reference implementation, not a production-hardened build. Several production paths named below (ACME/bring-your-own TLS, automated key rotation) are prod-deltas that are not present in the reference binary. Each recipe says so where it applies.

Pick your recipe:

- [Move from the dev certificate to production TLS](#recipe-move-from-the-dev-certificate-to-production-tls)
- [Cloak the origin so direct hits get a 421](#recipe-cloak-the-origin-so-direct-hits-get-a-421)
- [Stand up the admin listener with seeded role tokens](#recipe-stand-up-the-admin-listener-with-seeded-role-tokens)
- [Move a route from monitor to enforce](#recipe-move-a-route-from-monitor-to-enforce)
- [Set rate limits and work the ban ladder](#recipe-set-rate-limits-and-work-the-ban-ladder)
- [Not yet in the reference: a good-bot allowlist](#not-yet-in-the-reference-a-good-bot-allowlist)

---

## Recipe: move from the dev certificate to production TLS

**Goal:** understand what TLS the reference terminates, and what the path to production certificates is.

**Steps:**

1. Run Gate and observe the certificate it presents. The reference generates a self-signed certificate **in memory** on boot: ECDSA P-256, CN `humanymous.local`, with SANs `localhost`, `humanymous.local`, `127.0.0.1`, and `::1`. The negotiated TLS floor is version 1.2.
2. Expect a browser certificate warning when you visit `https://localhost:8444` in development. This is the self-signed dev cert, and the warning is normal for local use.
3. Plan production TLS as a prod-delta. Production terminates TLS with ACME-issued or bring-your-own certificates. **That path is not in the reference binary** — there is no flag in the reference build to load an operator-supplied certificate or key.

**Verify:** in development, inspect the presented certificate (for example with `openssl s_client`) and confirm the CN is `humanymous.local` and the SANs include `localhost` and `127.0.0.1`.

```
openssl s_client -connect localhost:8444 -servername localhost </dev/null 2>/dev/null | openssl x509 -noout -subject -ext subjectAltName
```

Expected (dev): a `subject=CN=humanymous.local` line and a SAN list containing `localhost`, `humanymous.local`, `127.0.0.1`, `::1`.

> **TODO(verify):** the exact production mechanism for supplying ACME or bring-your-own certificates (config field, file path, or wrapping component), since the reference binary exposes no CLI flag for it.

For the full TLS/ingress hardening table, see the [CLI, config & per-route policy reference](../reference/cli-config-policy.md).

---

## Recipe: cloak the origin so direct hits get a 421

**Goal:** make sure clients cannot skip Gate by talking to the origin directly. A direct-to-origin request should be refused with a `421` (hard rule HR-24), so all traffic is forced through the edge where it is scored and enforced.

**Steps:**

1. Choose a shared secret and pass it to Gate as a hex origin-cloaking HMAC key with `-origin-key`. When Gate proxies a request to the origin, it signs it with this key in the `X-Hmny-Origin-Auth` header.

   ```
   bin/gate.exe -addr :8444 -upstream http://127.0.0.1:9000 -origin-key <hex-key>
   ```

2. Configure your origin app to **require and validate** `X-Hmny-Origin-Auth` against the same key on every inbound request, and to reject requests that lack a valid value with `421 Misdirected Request`. Gate emits the signed header; the origin is what enforces the check.
3. Restrict origin network reachability as well (firewall/private network) so the origin is only addressable from Gate. The header check is the application-layer backstop; network isolation is the first line.

> **Note:** If you do not set `-origin-key`, the key is a random ephemeral value per boot. Cloaking only works end-to-end when the origin validates the same key you gave Gate, so a fixed, shared key is required for this recipe.

**Verify:** send a request straight to the origin, bypassing Gate. It should be refused.

```
curl -i http://127.0.0.1:9000/
```

Expected: `HTTP/1.1 421 Misdirected Request` from the origin (no valid `X-Hmny-Origin-Auth`). The same path fetched through the edge (`https://localhost:8444/`) is served normally. In the Ledger, a direct-origin bypass attempt surfaces as HR-24. See the [Hard rules, verdicts & signal-ID reference](../reference/hard-rules-verdicts.md).

> **TODO(verify):** whether the reference ships an example origin-side validator for `X-Hmny-Origin-Auth`, or whether the origin check is entirely the operator's responsibility.

---

## Recipe: stand up the admin listener with seeded role tokens

**Goal:** run the authenticated admin listener with the four RBAC roles, using deterministic tokens you control instead of random per-boot tokens.

**Steps:**

1. Choose one token string per role and set them in `HMN_ADMIN_TOKENS`, in the format `auditor:<tok>,operator:<tok>,approver:<tok>,dpo:<tok>`. Then start Gate; the admin listener defaults to `:8445` (a separate, cross-origin listener from the public edge).

   ```
   HMN_ADMIN_TOKENS="auditor:<tokA>,operator:<tokB>,approver:<tokC>,dpo:<tokD>" bin/gate.exe -addr :8444 -admin-addr :8445 -upstream http://127.0.0.1:9000
   ```

2. If you leave `HMN_ADMIN_TOKENS` unset, Gate mints **random tokens per boot** and prints them at startup. That is fine for a quick local look but changes on every restart; seed the variable when you want stable tokens.
3. Keep the roles separate. The four roles map to distinct capabilities: **Auditor** reads only; **Operator** reads and operates (request bans/erasure, lift bans, cancel erasure); **Approver** reads and approves (ban/kill-switch commits); **DPO** reads, operates, approves, and is the only role that can commit an erasure. Dual-control actions require the second action to come from a **distinct** role holder — so hand each token to a different person. See the [RBAC & separation-of-duties reference](../reference/rbac-separation-of-duties.md).

**Verify:** call `whoami` with one of your tokens and confirm the server-derived identity. Actor identity comes from the token, not from any request body.

```
curl -sk -H "Authorization: Bearer <tokB>" https://localhost:8445/__hmn/admin/whoami
```

Expected: a response identifying the Operator role. A missing or invalid token returns `404` (deny-by-default), not `401` — the admin surface is non-discoverable. Note that `/__hmn/admin/*` is **not** served on the public edge (`:8444` 404s it); it lives only on the admin listener.

Then open the Ledger at `https://localhost:8445/__hmn/admin/console` and walk the views with the [Ledger tour](./audit-console-tour.md).

---

## Recipe: move a route from monitor to enforce

**Goal:** take a route that is only scoring-and-logging and have it actually enforce verdicts (for example, promote a path from `monitor` to `balanced`, or to `strict`).

> **Important:** Route presets are set at **startup** via the route table (`Config.Routes` in `cmd/gate/main.go`). There is **no runtime per-route policy-write endpoint** in the reference. Changing one route's preset means editing the route table and restarting the process. The only runtime enforcement levers are the fleet-wide kill switch and the boot-global `-monitor` flag.

**Steps:**

1. Confirm the current per-route posture. Read it live from the Policy view in the Ledger, or via the admin API:

   ```
   curl -sk -H "Authorization: Bearer <auditor-tok>" https://localhost:8445/__hmn/admin/policy
   ```

2. Edit the route table in `cmd/gate/main.go` (`Config.Routes`). Set the target prefix to the preset you want — `monitor` (inject, score, log, no enforce), `balanced` (enforce, fail-open on an unknown safe GET/HEAD), or `strict` (enforce, fail-closed on unknown, synchronous re-score before state-changing actions). Routes match by **longest-prefix wins**; unmatched paths use `balanced`. Do not use `low` or `api` — those names are reserved and fall back to `balanced`.
3. Rebuild and restart Gate so the new route table takes effect.

   ```
   go build -o bin/gate.exe ./cmd/gate
   ```

4. Make sure you are not globally demoting enforcement. If Gate is running with `-monitor`, or the kill switch is active, **every** route is downgraded to monitor regardless of its preset. Remove `-monitor` for the route to enforce.

**Verify:** re-read `/__hmn/admin/policy` and confirm the route now shows the enforcing preset. Then send traffic that would score as automated and confirm the edge acts on it — a DENY blocks before the origin is contacted, a CHALLENGE serves the PoW interstitial. Watch the decision land in the Overview view of the Ledger; every decision is written to the audit log before it takes effect.

> **Tip:** Roll out enforcement the low-risk way — start a route in `monitor`, watch its would-be verdicts in Overview for a while, then promote it. See [Will this break my app?](../explanation/will-this-break-my-app.md).

---

## Recipe: set rate limits and work the ban ladder

**Goal:** understand the control-plane flood limiter and the auto-escalating ban ladder, and apply or lift a ban from the admin API.

**Steps:**

1. Know the flood limiter. A per-IP sliding-window limiter guards the control-plane endpoints `/__hmn/collect` and `/__hmn/session`. Reference defaults: **window 10s**, **soft threshold 60**, **hard threshold 120**. A hard breach returns `429` *before* scoring. These defaults are set from config fields (`Config.RateWindow` / `Config.RateSoft` / `Config.RateHard`) at startup, the same way the route table is — changing them means editing the config and restarting, not a runtime call.
2. Understand the auto-ban ladder. Repeated abuse escalates with strike decay: **1h → 6h → 24h → permanent**. Auto bans are recorded with `Source=auto`; bans you add by hand are `Source=manual`. Ban keys are `ip:<addr>` or `fp:<fingerprint>`.

   ```mermaid
   stateDiagram-v2
     [*] --> None
     None --> Ban1h: abuse (auto) or manual temp ban
     Ban1h --> Ban6h: re-offend
     Ban6h --> Ban24h: re-offend
     Ban24h --> Perm: re-offend
     Ban1h --> None: expiry / strike decay / lift
     Ban6h --> None: expiry / strike decay / lift
     Ban24h --> None: expiry / strike decay / lift
     Perm --> None: lift (single Operator)
     note right of Perm: manual permanent / CIDR ban = dual-control commit by a distinct Approver
   ```
3. Apply a temporary ban by hand. A temporary ban is a single Operator action (no second approver). Post it to the admin listener with the Operator token:

   ```
   curl -sk -X POST -H "Authorization: Bearer <operator-tok>" https://localhost:8445/__hmn/admin/bans -H "Content-Type: application/json" -d '{"Key":"fp:abc123","Reason":"scraper","DurationSec":3600}'
   ```

   A positive `DurationSec` on an `ip:`/`fp:` key applies immediately and returns `{"ok":true,"permanent":false}`. `DurationSec:0` (permanent) or a `cidr:` key instead returns a pending dual-control action for a distinct Approver.

4. Lift a ban. Unblocking is also a single Operator action, and it takes a **`?key=` query parameter, not a body**:

   ```
   curl -sk -X POST -H "Authorization: Bearer <operator-tok>" "https://localhost:8445/__hmn/admin/bans/lift?key=fp:abc123"
   ```

   Full admin request/response shapes are in the [CLI, config & policy reference](../reference/cli-config-policy.md#request--response-shapes).

**Verify:** list active bans and confirm your change:

```
curl -sk -H "Authorization: Bearer <auditor-tok>" https://localhost:8445/__hmn/admin/bans
```

Expected: the ban appears (or is gone, after a lift) with its key, source, and remaining duration; it is also visible in the Rate Limits & Bans view of the Ledger.

> **Warning:** Permanent and CIDR bans require **dual-control** — the commit must come from a distinct Approver, not the Operator who requested it. Bulk ban (`/__hmn/admin/bans/bulk`) is temporary-only; permanent and CIDR entries are rejected from bulk. For the full apply/approve/lift workflow, escalation, and the kill switch, see the [Kill switch & bans runbook](../runbooks/kill-switch-and-bans.md).

---

## Not yet in the reference: a good-bot allowlist

A good-bot allowlist (letting a named, trusted automated client through without challenge) is a **reserved concept** in this project. There is **no admin endpoint for an allowlist in the reference build**, and no working recipe for one. Do not configure or promise an allowlist as a shipping feature.

If you need to let known-good automation through today, the honest levers are the route table (an `off` or `monitor` preset on a specific prefix, set at startup) and the operational controls above — with the understanding that a preset applies to *all* traffic on that route, not to a specific client identity.

> **TODO(verify):** the intended production shape of a good-bot allowlist (identity model and admin surface), for readers planning ahead.

---

## Related pages

- [CLI, config & per-route policy reference](../reference/cli-config-policy.md) — every flag, preset, threshold, and endpoint.
- [Kill switch & bans runbook](../runbooks/kill-switch-and-bans.md) — the fleet-wide kill switch and the full ban workflow.
- [Key management](./key-management.md) — the sealed keystore (`-keystore` / `HMN_UNSEAL`), persistent identity, and why unsealed keys are ephemeral.
- [RBAC & separation-of-duties reference](../reference/rbac-separation-of-duties.md) — roles, capabilities, and dual-control.
- [How Gate sees a request](../concepts/how-gate-sees-a-request.md) — the L1–L7 pipeline and verdict flow behind these controls.
