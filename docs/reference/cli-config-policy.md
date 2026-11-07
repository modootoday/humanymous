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

### Observed Windows final-delivery boundary

At source `c57cf0c6`, one cooperative Windows parent test passed under the
separate controller at source `24004321`. Its four subcases observed exit codes
0, 74, 74 and 74 for normal completion, unread stdout pressure, unread stderr
pressure and late completion. Writer entry and release of the parent's pipe
ends were observed. The outer Windows Job controller observed the parent exit
and an empty Job within the original cleanup deadline.

These are four subcases of one parent test. Runtime completion was simulated;
the result does not prove a kernel-blocked write, SCM/account operations,
production native ownership or the full service lifecycle. The child report's
own cleanup fields remain separate from the outer controller's observations.

## Runtime startup and limits

Startup names `listen`, `origin`, `scope`, `routes`, `audit`, `owner`,
`session_capacity`, `session_ttl_ms`, `timeout_ms`, `rate_per_second`,
`session_rate_per_second` and optional `browser`.
Unknown fields are rejected. Both listeners/origins must be loopback; the origin
port is nonzero. Scope binds installation, tenant, origin, revision and authority.
Routes enumerate exact methods/paths and explicit required capabilities.

| Setting | Accepted range |
| --- | --- |
| `session_capacity` | 1–4096 |
| `session_ttl_ms` | 1000–3600000 milliseconds |
| `timeout_ms` | 1–30000 milliseconds |
| `rate_per_second` | 1–10000 |
| `session_rate_per_second` | 1–10000; defaults to 100 when omitted |
| Protected routes | 1–4096; exact supported methods and unescaped paths |

The global rate limits accepted connections before authentication. The session
rate applies after successful token consumption; a session-quota HTTP 429 can
therefore still consume its counter. Browser callers need the consumed-counter
ACK contract to interpret that result. These are resource limits, not detection
scores or evidence of automation.

`browser` is absent by default. When selected, its `origin` must exactly equal
`http://` followed by the configured scope authority. It enables the initial
cookie-free browser protocol policy, not browser collector or human qualification.

Each route defaults to `rit_profile: native_full` and
`response_profile: buffered`. Selecting `rit_profile: browser_auth_only` requires
the explicit browser policy; it rejects ambient cookies and uses the separate
browser-controlled binding/ACK contract. Selecting `response_profile: native_stream`
opts a native route into bounded incremental responses, with a 1 MiB response-body
limit. Combining browser integrity with native streaming rejects configuration.
The standalone listener remains plaintext; the separate TLS/H1 library path also
requires buffered native routes. See the [origin contract](gate-origin-contract.md)
for framing, credential forwarding and observed-versus-unqualified execution scope.

Every protected route requires request integrity. The plaintext standalone path
cannot satisfy required TLS capture. The optional TLS/H1 library path retains
its own handshaken connection owner; it does not recover original-client TLS
through a re-terminating proxy. HTTP/2 and qualified browser capture remain
unavailable, and routes requiring missing planes stay unready. A client assertion
never fills that capability gap, and declared readiness is not deployment proof.

Audit startup references a private durable directory and existing sealed custody
with its passphrase delivered privately through inherited configuration. Inline
HMAC/signing keys and unknown sources are rejected; startup does not silently
replace missing or malformed custody. Keep configuration, credentials and
visitor bodies out of public examples and logs.

See [installation requirements](install-requirements.md) and the
[origin contract](gate-origin-contract.md). Existing Go flags are not a Rust
compatibility promise.
