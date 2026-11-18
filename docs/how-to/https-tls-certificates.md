---
layout: doc
title: Plan TLS termination
permalink: /how-to/https-tls-certificates.html
reviewed: 2026-11-18
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

The runtime refuses to start, before it opens audit custody, when the `tls`
object names no source, more than one source, half a key pair, unreadable PEM
files, invalid domains or a source the build lacks. It never falls back to a
self-signed certificate. A provisioned pair is read once at startup; restart
the runtime to replace it. Rehearse ACME against the staging directory first;
staging certificates are not browser-trusted.

With ACME, the runtime renews in the background: it checks every interval,
renews 30 days before expiry (a third of the lifetime for shorter-lived
certificates) and backs off after failures. The certificate authority must
reach the listener for TLS-ALPN-01 validation.

**Limit:** the TLS listener binds and accepts loopback addresses only. To serve
public port 443, run a transport pass-through (TCP, not TLS-terminating)
forwarder on the same host to the loopback listener. The runtime does not yet
bind a public address itself.
