---
layout: doc
title: Plan TLS termination
permalink: /how-to/https-tls-certificates.html
reviewed: 2026-11-19
---
# Plan TLS termination

**Diátaxis quadrant:** How-to. **Audience:** infrastructure operators.

Draw every TLS hop and name the certificate/key owner at each endpoint. Original
client handshake observations require direct capture before normalization; a
CDN's connection to the origin is a different observation.

Keep private keys outside source and evidence records. Define renewal, reload,
expiry and failed-reload tests using synthetic lab certificates. Do not disable
verification to make a pilot succeed. See [topology boundaries](../reference/supported-topologies.md).

## Choose the runtime certificate source

The `humanymous-runtime` binary reads its settings from the private configuration
document on its standard input. Add a `tls` object to that document to request
the TLS listener; leave it out to keep the plaintext loopback listener. Give
exactly one source:

| Source | `tls` fields | Build |
|--------|--------------|-------|
| Your own certificate | `certificate` and `key` (PEM file paths, at most 1 MiB each) | any |
| ACME (Let's Encrypt by default) | `acme`: `domains`, `cache` (an owner-only directory), optional `email`, `directory`, `directory_root` and `check_interval_seconds` (default 3600) | `acme` feature |
| Development self-signed | `dev_self_signed: true` | `dev-self-signed` feature |

```json
"tls": {
  "acme": {
    "domains": ["gate.example.com"],
    "cache": "/var/lib/humanymous/acme",
    "directory": "https://acme-staging-v02.api.letsencrypt.org/directory"
  }
}
```

Once ready, the runtime's readiness line names the selected source in
`tls_mode` (`operator_certificate`, `acme` or `dev_self_signed`). It refuses to
start, before it opens audit custody, when the `tls`
object names no source, more than one source, half a key pair, unreadable PEM
files, invalid domains or a source the build lacks. It never falls back to a
self-signed certificate. A provisioned pair is read once at startup; restart
the runtime to replace it. Rehearse ACME against the staging directory first;
staging certificates are not browser-trusted.

With ACME, the runtime renews in the background: it checks every interval,
renews 30 days before expiry (a third of the lifetime for shorter-lived
certificates) and backs off after failures. The certificate authority must
reach the listener for TLS-ALPN-01 validation.

## Serve a public address

By default the listener binds and accepts loopback addresses only. To serve a
public address such as port 443 directly, set `listen` to that address and add
`"public_listen": true` beside a `tls` object with your own certificate or ACME.
The runtime refuses to start, before it opens audit custody, when a
non-loopback `listen` lacks `public_listen`, when `public_listen` has no `tls`
object, names the development certificate or sits on a loopback address, and
when it is combined with PROXY v2 client addresses. There is no public
plaintext listener. The protected origin stays a loopback address. The
readiness line then reports the `experimental-public-tls` profile with
`public_listen: true`; it remains an experimental profile, not product
qualification.

The optional administration listener (`admin` object) is always a separate
loopback address, even when the edge is public. Reach it from the same host or
through your own authenticated tunnel.

The Core role (`humanymous-runtime core`, which reads its own role document)
follows the same rules: loopback by default, and a public address only with
`public_listen` and a certificate or ACME source. It serves HTTP/1.1 only.
