---
title: HTTPS / TLS Certificates (Let's Encrypt, BYO, behind a terminator)
description: "How to give humanymous Gate a real, browser-trusted TLS certificate: automatic Let's Encrypt via TLS-ALPN-01 on :443, bring-your-own PEM, or self-signed behind an existing terminator — plus a staging dry-run, automatic renewal, and HSTS."
keywords: ["humanymous Gate TLS certificate","Let's Encrypt Docker","TLS-ALPN-01","acme-domain","acme-cache persistent volume","bring-your-own certificate","tls-cert tls-key","acme-directory staging","Let's Encrypt rate limits","HSTS","behind CDN nginx Caddy Traefik","trusted-proxies PROXY protocol","admin mTLS","automatic certificate renewal"]
---

# HTTPS / TLS certificates

**Diátaxis quadrant:** How-to. **Audience:** operators and integrators giving humanymous Gate ("Gate" after first mention) a real, browser-trusted certificate on a public domain.

Out of the box Gate terminates TLS with an **in-memory self-signed** certificate (CN `humanymous.local`), which is fine for local development but makes every browser warn. This page shows how to give the public edge a real certificate. Concepts and the full flag table are linked, not repeated: for every flag see the [CLI, config & per-route policy reference](../reference/cli-config-policy.md), and for where each detection layer is active in your topology see [Supported topologies](../reference/supported-topologies.md).

## Pick your certificate source

Gate's edge listener chooses its certificate from exactly one of three sources (`buildEdgeTLS` in `cmd/gate/tls.go`):

| Your situation | Use | Section |
|----------------|-----|---------|
| Public domain, Gate terminates TLS itself, nothing else on `:443` | **Let's Encrypt** (`-acme-domain`) — automatic issue + renew | [Let's Encrypt](#lets-encrypt-recommended) |
| You already have a certificate (corporate CA, wildcard, purchased) | **Bring-your-own** (`-tls-cert` / `-tls-key`) | [Bring-your-own](#bring-your-own-certificate) |
| A CDN / nginx / Caddy / Traefik / L7 LB already terminates TLS in front of Gate | **Self-signed / internal cert behind the terminator** + `-trusted-proxies` | [Behind a terminator](#behind-an-existing-tls-terminator--cdn) |
| Local development only | Do nothing — the self-signed dev cert is the default | — |

> **Note:** These sources apply to the **public edge** only. The **admin listener always stays self-signed** — it is a management plane, hardened with client-cert mTLS (`-admin-mtls-ca`), not ACME. See [Keep the admin plane on mTLS](#keep-the-admin-plane-on-mtls-not-acme).

---

## Let's Encrypt (recommended)

`-acme-domain` requests a free, browser-trusted Let's Encrypt certificate and renews it automatically. Gate answers the ACME challenge with **TLS-ALPN-01**, which is answered **inline during the TLS handshake on `:443`** — there is **no port 80 and no HTTP-01**.

### Prerequisites

1. **DNS.** An `A` (and/or `AAAA`) record for your domain points at the host's public IP.
2. **Inbound `:443` open from anywhere.** Let's Encrypt validation servers connect from varying IPs, so `:443` must be reachable from the public internet, not just your office.
3. **Nothing else terminates TLS for that domain on `:443`.** TLS-ALPN-01 is answered in the handshake, so no CDN, WAF, or L7 load balancer may re-terminate TLS in front of Gate. (If something must, you are in the [behind-a-terminator](#behind-an-existing-tls-terminator--cdn) topology, not this one.)
4. **A persistent volume for the certificate cache** (see the caution below).

### Run it (single container)

Map host `:443` to the container's edge port (`:8444`) and mount a **named volume** for the ACME cache:

```bash
docker run -d \
  -p 443:8444 -p 127.0.0.1:8445:8445 \
  -v hmn-acme:/acme-cache \
  ghcr.io/modootoday/humanymous-gate:latest \
  -addr :8444 -admin-addr :8445 \
  -upstream http://YOUR-ORIGIN:PORT \
  -acme-domain your.domain \
  -acme-cache /acme-cache \
  -acme-email you@your.domain \
  -monitor
```

- `:latest` tracks the newest release.
- `-monitor` scores and logs without enforcing — the safe first contact with real traffic. Drop it to enforce.
- `-acme-email` is optional but recommended: Let's Encrypt uses it for expiry and policy notices.

> **Caution — the cache MUST be an absolute path on a persistent, writable volume.** The default `-acme-cache` value is *relative* (`acme-cache` → `/app/acme-cache`), which lands on the container's **read-only root filesystem** and is not persisted. That breaks issuance outright, and even if it did not, it would re-issue on every restart and quickly exhaust Let's Encrypt's rate limits. Always pass an absolute path (`-acme-cache /acme-cache`) mounted on a named volume (`-v hmn-acme:/acme-cache`). The published gate image pre-creates `/acme-cache` and `/data` writable by the non-root uid.

### Or use the pull-only Compose file

`deployments/compose.release.yaml` already wires this correctly — `-acme-domain ${HMN_DOMAIN}`, `-acme-cache /acme-cache` onto the `hmn-acme` named volume, and `443:8444` published:

```bash
cd deployments
cp .env.example .env            # set HMN_UPSTREAM, HMN_DOMAIN, HMN_UNSEAL, HMN_ADMIN_TOKENS
cp routes.conf.example routes.conf
docker compose -f compose.release.yaml up -d
```

Set `HMN_DOMAIN` to your public domain. For multiple domains on one certificate, set a comma-separated SAN list (see [Multi-domain](#multi-domain-san-certificates)). See the full runbook in [Deployment & policy operations](./deployment-policy-operations.md#recipe-deploy-from-the-published-image-with-pull-only-compose).

### Verify

Issuance happens on the first inbound TLS handshake for the domain, so hit the edge once, then inspect the presented certificate:

```bash
openssl s_client -connect your.domain:443 -servername your.domain </dev/null 2>/dev/null \
  | openssl x509 -noout -issuer -subject -dates
```

Expected: an `issuer=` line naming Let's Encrypt (`R3` / `E1` or the current intermediate), `subject=CN=your.domain`, and valid `notBefore`/`notAfter` dates. In a browser, the padlock shows no warning. If you still see CN `humanymous.local`, ACME has not completed — see [Troubleshooting](#troubleshooting).

### Staging dry-run (get DNS and firewall right without burning rate limits)

Before your first production issuance, dry-run against the **Let's Encrypt staging** directory with `-acme-directory`. Staging has far looser rate limits, so you can iterate on DNS and firewall without risking a production lockout:

```bash
docker run -d \
  -p 443:8444 -p 127.0.0.1:8445:8445 \
  -v hmn-acme-staging:/acme-cache \
  ghcr.io/modootoday/humanymous-gate:latest \
  -addr :8444 -admin-addr :8445 \
  -upstream http://YOUR-ORIGIN:PORT \
  -acme-domain your.domain \
  -acme-cache /acme-cache \
  -acme-directory https://acme-staging-v02.api.letsencrypt.org/directory \
  -monitor
```

Staging certificates are signed by an **untrusted root**, so browsers will still warn — that is expected; you are validating the issuance flow, not the trust chain. `openssl` above will show a `(STAGING)` issuer. Once staging succeeds, **switch back to production** by dropping `-acme-directory` (empty = production Let's Encrypt) and use a **fresh cache volume** (e.g. the `hmn-acme` volume, not `hmn-acme-staging`) so a staging cert is not reused. `-acme-directory` also accepts a ZeroSSL directory URL or an internal step-ca URL.

### Automatic renewal

Renewal is **automatic** — autocert renews roughly 30 days before expiry, in-process, as long as `:443` stays reachable and the cache volume persists. There is **no cron and no certbot** to run. Keep the cache volume mounted across restarts and upgrades so renewed certs (and account keys) survive.

### Rate limits (production Let's Encrypt)

Production Let's Encrypt enforces (approximately): **~50 certificates per registered domain per week**, **5 duplicate certificates per week**, and per-account/hostname failed-validation limits per hour. This is why the persistent cache and the staging dry-run matter — a misconfigured cache that re-issues on every restart will hit these limits fast. Use [staging](#staging-dry-run-get-dns-and-firewall-right-without-burning-rate-limits) while getting DNS and firewall right.

### Multi-domain (SAN) certificates

Pass a comma-separated list to put several names on one certificate (autocert `HostWhitelist`):

```
-acme-domain www.example.com,example.com,api.example.com
```

Every listed name must have DNS pointing at the same host and resolve to a reachable `:443`.

---

## Bring-your-own certificate

If you already hold a certificate — a wildcard, a purchased cert, or one from a corporate/internal CA — hand Gate the PEM files with `-tls-cert` and `-tls-key`. No ACME, no `:443` challenge requirement.

Mount the PEMs **read-only** and point the flags at them:

```bash
docker run -d \
  -p 443:8444 -p 127.0.0.1:8445:8445 \
  -v /etc/ssl/your-cert.pem:/certs/tls.crt:ro \
  -v /etc/ssl/your-key.pem:/certs/tls.key:ro \
  ghcr.io/modootoday/humanymous-gate:latest \
  -addr :8444 -admin-addr :8445 \
  -upstream http://YOUR-ORIGIN:PORT \
  -tls-cert /certs/tls.crt \
  -tls-key /certs/tls.key \
  -monitor
```

Both flags are required together; the certificate PEM should include any intermediate chain. The negotiated TLS floor is version 1.2.

**Renewal is your responsibility:** replace the PEM files and restart Gate. Restart is safe — on `SIGINT`/`SIGTERM` Gate drains in-flight requests within `-shutdown-grace` (default 25s) before exiting, so run the restart within your orchestrator's termination grace. Certificates are read at boot, so a rotated file only takes effect after a restart.

---

## Behind an existing TLS terminator / CDN

If a CDN, nginx, Caddy, Traefik, or an L7 load balancer already terminates TLS in front of Gate, **do not run ACME on Gate** — the terminator, not Gate, owns the public certificate. Run Gate with its default self-signed dev certificate (or your own internal cert via `-tls-cert`/`-tls-key`) behind the terminator, and tell Gate which balancers to trust so it can recover the real client IP:

```
-trusted-proxies <cidr-of-your-terminator>
```

Set `-trusted-proxies` to your terminator's addresses **only** — never a broad range like `0.0.0.0/0`, or any client could spoof its source IP. With it set, Gate reads the real client IP from the PROXY-protocol-v2 header the L4 balancer sends, keeping IP-keyed state (bans, rate limits, correlation) correct.

> **Caveat — the TLS fingerprint plane goes inert behind a re-terminator.** When anything re-terminates TLS in front of Gate, the ClientHello Gate sees is the *terminator's*, not the browser's, so the JA3/JA4/HTTP-2 fingerprint plane (HR-2/HR-5/HR-11/HR-14) is **inactive and silent** — detection collapses to the client, header, behavior, and IP-intel planes. For TLS fingerprinting you must terminate **raw TLS at the Core engine with nothing in front**. This is a placement decision, not a tunable; read [Supported topologies](../reference/supported-topologies.md) before you choose this shape.

---

## Enable HSTS after the cert is stable

Once a real certificate is issued and stable, add `Strict-Transport-Security` with `-hsts`:

```
-hsts
```

Enable it **only after** the real cert is confirmed working. HSTS pins the browser to HTTPS for the max-age window, so a bad, expired, or accidentally-self-signed certificate then **hard-blocks** browsers with no click-through. Leave `-hsts` off while you are still on the dev cert or dialing in ACME.

## Keep the admin plane on mTLS (not ACME)

The `-acme-domain` / `-tls-cert` sources configure the **public edge only**. The admin listener (`-admin-addr`, default `127.0.0.1:8445`) always keeps its own self-signed certificate — it is a management plane, not a public surface. Harden it with **client-cert mTLS** using `-admin-mtls-ca` (a PEM of client-cert CA(s)); when set, the admin listener requires a client certificate signed by that CA **in addition to** the bearer token. Do not point ACME at the admin listener. See [Deployment & policy operations](./deployment-policy-operations.md#recipe-stand-up-the-admin-listener-with-seeded-role-tokens).

---

## Troubleshooting

Common ACME (Let's Encrypt) failures and what they mean:

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Cert never issues; edge still presents CN `humanymous.local` | `:443` is not reachable from the public internet | Open inbound `:443` in the host/cloud firewall and security groups; TLS-ALPN-01 uses **only** `:443` (no port 80). |
| Issuance fails; validation errors | DNS `A`/`AAAA` record does not point at this host's public IP | Fix the DNS record and let it propagate, then retry. |
| Cert issues but as the *terminator's*, or handshake fails | Something else terminates TLS for the domain on `:443` in front of Gate | Remove the re-terminating CDN/LB, or switch to the [behind-a-terminator](#behind-an-existing-tls-terminator--cdn) topology (no ACME on Gate). |
| Issuance blocked / "too many certificates" | Hit a **production Let's Encrypt rate limit** | Switch to the [staging directory](#staging-dry-run-get-dns-and-firewall-right-without-burning-rate-limits) with `-acme-directory` until DNS/firewall are correct, then go back to production with a fresh cache. |
| New cert on every restart / rate-limited quickly | `-acme-cache` is not persisted (relative default on read-only rootfs) | Pass an absolute `-acme-cache /acme-cache` on a named volume (`-v hmn-acme:/acme-cache`); confirm the volume survives restarts. |
| Browser hard-blocks after a cert problem | `-hsts` was enabled before the cert was stable | HSTS pins HTTPS for its max-age; fix the certificate. Only enable `-hsts` after the cert is confirmed. |

---

## Related pages

- [Deployment & policy operations](./deployment-policy-operations.md) — the full deploy runbook, including moving from the dev cert to production TLS.
- [Supported topologies](../reference/supported-topologies.md) — which detection layers are active where, and why a re-terminator makes the TLS plane inert.
- [CLI, config & per-route policy reference](../reference/cli-config-policy.md) — every TLS/ACME flag, `-trusted-proxies`, `-hsts`, and `-admin-mtls-ca`.
- [Install requirements](../reference/install-requirements.md) — ports, the one dependency, and running from the published image.
