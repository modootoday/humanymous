---
layout: doc
title: Command and configuration contract
permalink: /reference/cli-config-policy.html
reviewed: 2026-11-07
---
# Command and configuration contract

**Diátaxis quadrant:** Reference. **Audience:** operators and adapter authors.

Three different executables have distinct responsibilities. None establishes
a qualified production installation by its presence alone.

| Executable | Implemented interface | Boundary |
| --- | --- | --- |
| `humanymous` | `doctor`, `install`, `rollback`; `--artifact`, `--destination`, `--sha256`, `--local-management-verified`, optional `--dry-run` | Local artifact provisioning; no elevation, service registration, activation or release-signature verification |
| `humanymous-native` | `init --framework`, optional `--target`, `--runtime-version`, `--workspace`, `--output`, `--dry-run` or `--write`; `--help`, `--version` | Integration manifest planning, not an implemented framework SDK; `doctor` and `verify` are not current commands |
| `humanymous-runtime` | Bounded JSON startup document on inherited stdin, then one JSON readiness line on stdout | Experimental loopback HTTP/1.1; no public management listener |

## Runtime startup and limits

Startup names `listen`, `origin`, `scope`, `routes`, `audit`, `owner`,
`session_capacity`, `session_ttl_ms`, `timeout_ms` and `rate_per_second`.
Unknown fields are rejected. Both listeners/origins must be loopback; the origin
port is nonzero. Scope binds installation, tenant, origin, revision and authority.
Routes enumerate exact methods/paths and explicit required capabilities.

| Setting | Accepted range |
| --- | --- |
| `session_capacity` | 1–4096 |
| `session_ttl_ms` | 1000–3600000 milliseconds |
| `timeout_ms` | 1–30000 milliseconds |
| `rate_per_second` | 1–10000 |
| Protected routes | 1–4096; exact supported methods and unescaped paths |

Every protected route requires request integrity. Required original TLS,
HTTP/2 or qualified browser capture is unavailable in this slice, so those
routes cannot become ready. A client assertion never fills that capability gap.

Audit startup references a private durable directory and existing sealed custody
with its passphrase delivered privately through inherited configuration. Inline
HMAC/signing keys and unknown sources are rejected; startup does not silently
replace missing or malformed custody. Keep configuration, credentials and
visitor bodies out of public examples and logs.

See [installation requirements](install-requirements.md) and the
[origin contract](gate-origin-contract.md). Existing Go flags are not a Rust
compatibility promise.
