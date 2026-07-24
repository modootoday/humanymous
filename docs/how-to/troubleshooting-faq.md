---
title: Integration Troubleshooting & FAQ
description: "Symptom-to-fix guide for humanymous Gate integration errors: self-signed cert warnings, HTTP 421 origin bypass, admin API 404, CHALLENGE'd users, and keystore boot. Reference implementation."
keywords: ["humanymous Gate troubleshooting","HTTP 421 origin bypass","self-signed certificate warning","admin API 404 bearer token","detection bundle not injecting","keystore boot failure HMN_UNSEAL","CHALLENGE false positive tuning","port already in use","proof-of-work interstitial","reverse-proxy bot detection"]
---

# Integration troubleshooting & FAQ

**Diátaxis quadrant:** How-to (FAQ). **Audience:** integrators standing up humanymous Gate for the first time and working through the errors that show up mid-integration.

This page is a symptom → cause → fix reference for the problems you are most likely to hit while wiring humanymous Gate ("Gate" after first mention) in front of your origin. Gate is the reverse-proxy enforcement layer: it terminates TLS, streams the detection bundle into your HTML, scores layers L1–L7 inline, enforces a verdict (ALLOW / CHALLENGE / DENY) at the edge, and writes every decision to a tamper-evident audit log.

Everything below is written against the reference implementation. Some behaviors you might expect from a production install (real certificates, shared fleet state, and more) are intentionally not in the reference — those are called out as prod-delta. For the full boundary, see [Production vs reference](../reference/production-vs-reference.md).

For flags, environment variables, presets, and defaults referenced throughout, see [CLI, config & policy reference](../reference/cli-config-policy.md).

---

## Your browser shows a certificate warning

**Symptom.** You open the edge listener in a browser and get a security warning ("your connection is not private", self-signed / untrusted issuer), or a command-line client refuses the TLS handshake.

**Cause.** The reference implementation serves a self-signed certificate generated in memory. It is not issued by a trusted authority, so browsers and default TLS clients reject it. This is expected in development.

**Fix.** In development, tell your client to skip certificate verification. With `curl`, use `-k`:

```
curl -k https://localhost:8444/
```

For a browser, proceed through the warning (or trust the dev certificate locally). Do not treat the warning as a Gate fault — it is the expected state of the in-memory dev certificate.

> **Note:** Real certificates (ACME / a managed CA) and a real KMS/HSM for key material are prod-delta — the reference does not ship them. Plan certificate issuance as part of your production deployment; see [Production vs reference](../reference/production-vs-reference.md).

---

## Requests come back with HTTP 421

**Symptom.** Some requests to your origin return `421`.

**Cause.** A `421` is hard rule HR-24: a request reached the origin directly, bypassing Gate. Once you set `-origin-key`, Gate signs traffic it forwards with the `X-Hmny-Origin-Auth` header (origin cloaking), and the origin validates that header. A request that arrives without a valid `X-Hmny-Origin-Auth` — that is, one that did not pass through the edge — is rejected with `421`.

**Fix.** Route all client traffic through the Gate edge listener (default `:8444`) rather than letting clients reach the origin (default `:9000`) directly. Confirm:

- `-origin-key` is set to the same value that the origin uses to validate `X-Hmny-Origin-Auth`.
- Your load balancer, DNS, or firewall sends external traffic to the edge, and the origin is only reachable from Gate.

The `-origin-key` value is separate from the sealed keystore; see [Key management, rotation & recovery](./key-management.md) for how it fits alongside the other key material.

---

## The detection bundle is not appearing in my page

**Symptom.** You expected Gate's control-plane script to be injected, but the page source that reaches the browser has no injected `<script>`, and no beacon reaches `/__hmn/collect`.

**Cause.** Streaming HTML injection is single-pass, add-only, idempotent, and applies to **HTML responses only**. Gate does not inject into responses that are not HTML — for example non-HTML content types, or responses it cannot treat as injectable HTML in a single streaming pass.

**Fix.** Confirm the response you expect injection on is served as HTML (an HTML `Content-Type`). Static assets, JSON APIs, and other non-HTML responses are not injected by design. If a page that should be HTML is not getting the bundle, verify the origin is returning it as an HTML response rather than, for example, a downloadable or opaque body.

> **Note:** A page that is served the bundle but never sends a beacon is itself a signal — HR-25 (bundle injected but no beacon) results in a CHALLENGE. If you are testing and see challenges on a freshly injected page, confirm the control-plane script is actually loading and posting to `/__hmn/collect`.

For how the bundle, the beacon, and the `/__hmn/*` control plane fit together, see [The control plane and the detection bundle](../explanation/control-plane-and-bundle.md).

In the returned HTML, look for this exact markup (inserted once at the first `<head>` boundary):

```
<!--hmn-injected--><script src="/__hmn/loader.js" defer></script>
```

If you see the `<!--hmn-injected-->` marker but no bundle behavior, the page was injected but the loader may be blocked by your CSP (see [The control plane and the injected bundle](../explanation/control-plane-and-bundle.md)). If you see neither, the response was not treated as HTML, or the head boundary was beyond the 64 KiB look-ahead.

---

## Legitimate users are getting CHALLENGE'd

**Symptom.** Real people hit the proof-of-work (PoW) challenge interstitial when you did not expect them to.

**Cause.** When Gate cannot form a verdict (Unknown), it **fails closed** on strict routes and on all unsafe methods (POST, PUT, PATCH, DELETE) — those requests are challenged rather than passed. This is deliberate: the default policy routes sensitive paths such as `/login`, `/checkout`, and `/admin` to the `strict` preset. It fails open (passes) only for safe `GET`/`HEAD` requests on non-strict routes, a documented, accepted residual covered by fingerprint and subnet rate metering. Some heuristic hard rules can also challenge real people — for example HR-12 (no interaction over the window) is explicitly a heuristic that can catch some humans.

**Fix.**

- Start in monitor mode to observe verdicts without enforcing them, then tune before you enforce. Monitor mode lets you see what *would* be challenged or denied against real traffic first.
- Rely on the PoW upgrade path: a score-based CHALLENGE (one with no hard rule behind it) upgrades to ALLOW once the client solves the proof of work (`l7.pow.solved`). A real human completing the interstitial passes. The PoW upgrade never overrides a hard rule. On an **`attested`** route this bare solve is not enough: the solve must **also** mint a step-up receipt that redeems at `POST /__hmn/stepup` for an `hmn_su` cookie, or the attestation floor keeps pricing the ALLOW back to a Pass (see the FAQ entry below).
- Review which routes are mapped to `strict`. Presets are startup configuration (`Config.Routes`); there is no runtime per-route policy-write endpoint. Adjust the route-to-preset mapping at startup if a route is stricter than you need.

For a fuller treatment of what changes for real users and how to de-risk enforcement, see [Will this break my app?](../explanation/will-this-break-my-app.md).

---

## The Ledger will not authenticate, or returns 404

**Symptom.** The Ledger (or an admin API call) returns `404`, or your bearer token is not accepted.

**Cause.** Two things commonly cause this:

1. **Deny-by-default auth.** Admin authentication is bearer-token, compared in constant time. A missing or invalid token returns `404` (not `401`) — the endpoint does not reveal itself to an unauthenticated caller.
2. **Wrong listener.** The Ledger and the whole admin API live on a **separate admin listener** (default `127.0.0.1:8445` (loopback)) at `/__hmn/admin/`. On the public edge listener, `/__hmn/admin/*` returns `404` by design. If you are calling the admin path on the edge port, you will always get `404`.

**Fix.**

- Use the admin listener. The console is served at `https://localhost:8445/__hmn/admin/console`, and the admin API base is `/__hmn/admin`.
- Present a valid token. If you did not set `HMN_ADMIN_TOKENS`, Gate generates random tokens per boot and prints them at startup — read them from the startup log. To set them yourself, use `HMN_ADMIN_TOKENS`:

```
HMN_ADMIN_TOKENS="auditor:<tok>,operator:<tok>,approver:<tok>,dpo:<tok>" bin/gate.exe -addr :8444 -admin-addr :8445
```

- Send the token as a bearer credential, for example:

```
curl -k -H "Authorization: Bearer <operator-token>" https://localhost:8445/__hmn/admin/whoami
```

Every admin access is meta-audited (`admin.access`), and the actor identity is derived by the server from the token — a body-supplied actor is ignored. Role capabilities differ (Auditor / Operator / Approver / DPO); see [RBAC & separation of duties](../reference/rbac-separation-of-duties.md).

---

## "Port already in use" on startup

**Symptom.** Gate fails to bind a listener at boot with an address-in-use error.

**Cause.** A default port is already taken. The reference uses three:

| Listener | Default | Flag |
|----------|---------|------|
| Edge (public) | `:8444` | `-addr` |
| Admin | `:8445` | `-admin-addr` |
| Origin / upstream | `:9000` (as `http://127.0.0.1:9000`) | `-upstream` |

**Fix.** Free the conflicting port, or point the relevant flag at a different one. For example, to move the edge and admin listeners:

```
bin/gate.exe -addr :18444 -admin-addr :18445 -upstream http://127.0.0.1:9000
```

The origin address comes from your `-upstream` target; make sure that host and port are the one your origin app actually listens on.

---

## Keystore boot failure

**Symptom.** Gate exits at startup instead of coming up, when you have configured a keystore.

**Cause.** When `-keystore` is set, the `HMN_UNSEAL` environment variable is **required** — the passphrase opens (or creates) the sealed file. Boot fails if `-keystore` is set and `HMN_UNSEAL` is unset.

**Fix.** Provide the passphrase in the environment alongside the flag:

```
HMN_UNSEAL="<passphrase>" bin/gate.exe -keystore /var/lib/gate/keystore.sealed -addr :8444 -admin-addr :8445
```

> **Warning:** Losing `HMN_UNSEAL` means you cannot open the sealed identity, which is equivalent to a mass cryptographic erasure (crypto-shred) of the linkage it protects. Back the passphrase up out-of-band. See [Key management, rotation & recovery](./key-management.md).

---

## A restart lost all pseudonym linkage, or the verifier public key changed

**Symptom.** After a restart, pseudonyms no longer resolve to the same subjects, and/or the public key your audit-log verifier expects no longer matches.

**Cause.** You are running without a keystore, so the keys are **ephemeral** (held in memory only). On restart, Gate mints a new signing key — so the verifier's expected public key changes — and creates a new vault, so the per-subject pseudonym linkage from before the restart is lost (effectively a mass crypto-shred).

**Fix.** Persist node identity by running with a sealed keystore. Set both `-keystore` and `HMN_UNSEAL`:

```
HMN_UNSEAL="<passphrase>" bin/gate.exe -keystore /var/lib/gate/keystore.sealed -addr :8444 -admin-addr :8445
```

With the keystore, the SigningSeed (Ed25519 STH key), HMACKey, and vault snapshot persist across restarts, so the verifier public key stays stable and pseudonym linkage survives. Full detail — what each key protects, sealing (`scrypt` N=2^15 + AES-256-GCM), and recovery — is in [Key management, rotation & recovery](./key-management.md).

---

## Quick FAQ

**Is the self-signed certificate a bug?** No. The reference generates it in memory; use `curl -k` in development. Real certificates are prod-delta.

**Why `404` instead of `401` from the admin API?** Deny-by-default: a missing or invalid bearer token returns `404` so the admin surface does not announce itself. Check both the token and that you are on the admin listener (`:8445`), not the edge.

**Can I change route policy while Gate is running?** Presets (`off` / `monitor` / `balanced` / `strict` / `attested`) are startup configuration. There is no runtime per-route policy-write endpoint. The runtime levers are the fleet-wide kill switch and the global `-monitor` switch.

**How do I stop enforcement without stopping traffic?** The kill switch demotes hard-rule enforcement to monitor fleet-wide — detection stops and traffic flows, though manually placed bans still enforce. It is dual-control (committed by a distinct Approver).

> **Warning:** The kill switch is fleet-wide. Enabling it stops hard-rule enforcement everywhere Gate runs, not just on one node.

**Why is an ALLOW still challenged on my high-value (`attested`) route?** That is the attestation floor (ceiling-guard #1) doing its job: on an `attested` route a scoring-ALLOW — and even a valid verdict-token fast-path — is priced to a Pass challenge unless the session presents possession or an `hmn_su` step-up proof. The fix is to give humans a way to satisfy the floor: either wire a credential verifier (WebAuthn / Privacy Pass / Web Bot Auth) so possession pre-gates and forwards first, **or** run the Core Pass server and Gate with a **shared `HMN_TOKEN_KEY`** and the same cookie jar so a Pass solve's receipt redeems at `POST /__hmn/stepup` and mints `hmn_su`. Without one of those, humans re-solve the Pass forever — a Pass-for-everyone friction wall. (Note: `attested` also requires the shared `HMN_TOKEN_KEY` across Core and Gate to start at all; the Gate refuses to boot without it.)

**Where do I see what Gate decided?** Observability is the audit stream (`GET /audit`), the Integrity view/endpoint, and the Overview KPIs in the Ledger, plus a Prometheus `GET /__hmn/admin/metrics` gauge snapshot on the admin plane and the `GET /__hmn/healthz` (liveness) / `GET /__hmn/readyz` (readiness) probes answered by Gate itself. Per-verdict rates come from the audit stream; SIEM shipping of that stream is the prod-delta.

---

## Related pages

- [CLI, config & policy reference](../reference/cli-config-policy.md) — flags, environment variables, presets, defaults.
- [Will this break my app?](../explanation/will-this-break-my-app.md) — what enforcement changes for real users, and how to de-risk it.
- [The control plane and the detection bundle](../explanation/control-plane-and-bundle.md) — how injection, the beacon, and `/__hmn/*` work.
- [Key management, rotation & recovery](./key-management.md) — the keystore, `HMN_UNSEAL`, `-origin-key`, and what breaks when key material is lost.
- [Production vs reference](../reference/production-vs-reference.md) — the full prod-delta boundary.
