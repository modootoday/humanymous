---
layout: doc
title: Review credential verification boundaries
permalink: /how-to/configure-credential-verifiers.html
reviewed: 2026-11-21
---
# Review credential verification boundaries

**Diátaxis quadrant:** How-to. **Audience:** identity and application integrators.

This is not qualified product configuration. Define who issued a credential,
what it authorizes, its audience, expiry, request binding and revocation behavior.
A valid signature does not grant unlimited origin access or prove human presence.

Test wrong issuer/audience, expiration, replay, cross-tenant use and unavailable
verification dependencies. Bound remote discovery against redirects, oversized
responses and private-network targets. Keep adapter credentials separate from
administrative authority. See [administrative roles](../reference/rbac-separation-of-duties.md).

## Configure gateway trust anchors

The runtime gateway can verify two credential types when its private
configuration has a `credentials` object:

```json
"credentials": {
  "agent_keys": "/srv/humanymous/credentials/agent-keys.txt",
  "pat_issuers": "/srv/humanymous/credentials/pat-issuers.pem"
}
```

- `agent_keys` is a Web Bot Auth allowlist: one `keyid key` pair per line,
  where `key` is the agent's Ed25519 public key in unpadded base64url. Blank
  lines and lines starting with `#` are ignored.
- `pat_issuers` holds the PEM `PUBLIC KEY` blocks of the Privacy Pass issuers
  you trust (RSA keys of 2048 to 8192 bits).

Either one may be left out, but the object must name at least one file. The
gateway reads both files once at startup. It refuses to start when a file
cannot be read, is larger than 4 MiB, has a malformed line, a duplicate
identifier or an invalid key, or contains no key at all. The readiness line
reports which verifiers are on under `profile.credentials`; it never shows
paths or keys.

A verified credential is checked only after the request's integrity checks
pass, and it then lets the request through without scoring. It never changes
a score, threshold or signal. Treat each listed key as a grant of that
bypass, and remove keys you no longer trust, then restart the gateway.
WebAuthn credentials cannot be configured on the gateway yet.
