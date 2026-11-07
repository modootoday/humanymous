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
| `humanymous` | Provisioning `doctor`, `install`, `rollback`; separate signed `service-*` commands below | Explicit local provisioning or owned lab-service lifecycle; no elevation or full product qualification |
| `humanymous-native` | `init --framework`, optional `--target`, `--runtime-version`, `--workspace`, `--output`, `--dry-run` or `--write`; `--help`, `--version` | Integration manifest planning, not an implemented framework SDK; `doctor` and `verify` are not current commands |
| `humanymous-runtime` | Bounded JSON startup document on inherited stdin, then one JSON readiness line on stdout | Experimental loopback HTTP/1.1; no public management listener |

## Native integration planner outcomes

For `humanymous-native`, default/explicit dry-run does no workspace inspection.
Explicit Windows manifest writing checks path/reparse boundaries and pins
directory handles; it does not verify workspace owner/ACL policy. It requires
trusted, non-hostile workspace entries. Publication is no-clobber, with separate
temporary cleanup reporting: exit 0 means normal completion, exit 2 refusal or failed publication,
and exit 3 a published manifest with incomplete temporary cleanup. Inspect
`publication.published`, `cleanup_complete` and `temporary_file` before retrying.
On failed publication, stderr reports staging cleanup and the temporary basename;
inspect it because a temporary file can remain even without destination creation.
These codes concern the planner, not the provisioning/service executable below.

## Provisioning and signed service commands

The original `doctor`, `install` and `rollback` commands require `--artifact`,
`--destination`, `--sha256` and `--local-management-verified`; `--dry-run` is
optional. These commands manage one artifact and receipt, without registering
or starting a service. Their digest check is separate from release signing.

The signed service interface uses an independently pinned public key and exact
descriptor bytes. It is an owned laboratory lifecycle, not a qualified Core/Gate
installer. See [installation boundaries](install-requirements.md).

| Command | Required options and behavior |
| --- | --- |
| `service-plan` | `--manifest`, `--signature`, `--trust-key`, `--artifact`, `--destination`; read-only, rejects `--activate` |
| `service-install`, `service-upgrade` | Same five options; `--activate` is required for mutation; `--dry-run` describes the plan without mutation |
| `service-doctor` | Only `--destination` and `--trust-key`; queries actual native manager and loopback diagnostic readiness; rejects activation/dry-run flags |
| `service-rollback`, `service-remove` | Only destination/key plus `--activate` for mutation or `--dry-run` for a read-only plan |

`--activate` and `--dry-run` are mutually exclusive. Unknown or duplicate options
fail. Signature and trust key are lowercase hexadecimal strings, respectively
128 and 64 characters. No service command fetches a trust key from the network.
Successful service diagnostics retain `product_qualified: false`; a ready
fixture is not authenticated runtime, recovery or management qualification.

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
