<p align="center">
  <img src="docs/assets/brand/lockup.svg" width="420" alt="humanymous — raise the cost of automation">
</p>

<p align="center">
  Apache-2.0 reference implementation for browser-automation detection, reverse-proxy enforcement, and tamper-evident operational audit.
</p>

# humanymous

humanymous raises the cost of automated access to an application you operate. It combines browser evidence, server observations, consistency checks, request metering, and edge protections to produce an ALLOW, CHALLENGE, or DENY verdict.

This repository is a reference implementation, not a production-hardened service. It does not claim to identify every automated session or to eliminate false positives.

[Read the documentation](docs/README.md) · [Start with the glossary](docs/reference/glossary.md) · [Review the detection boundary](docs/explanation/what-gate-is.md)

## Two execution surfaces

The repository contains two binaries that share scoring code but do not collect identical evidence.

| Surface | Purpose | Evidence available |
|---|---|---|
| **Core detection engine** | Development, reference measurement, and local inspection | Full browser detector plus direct network-protocol observations when Core receives the original encrypted connection |
| **humanymous Gate** | Reverse-proxy enforcement in front of an origin application | A narrower browser beacon, request headers, source and rate state, route policy, and edge protections |

Core's measurements are not Gate's measurements. Do not use a Core catalog result as a claim about Gate.

The **Ledger** is Gate's operator console. The **Detection Observatory** is Core's local-only developer view.

## How evaluation works

Core's complete detection pipeline has seven named stages:

1. Static client inspection
2. Client fingerprinting
3. Client integrity checks
4. Interaction analysis
5. Network and protocol inspection
6. Consistency checks
7. Risk aggregation and verdict selection

The result is a risk score from zero to one hundred. The built-in defaults map scores below 30 to ALLOW, scores from 30 through 69 to CHALLENGE, and scores from 70 upward to DENY. Named enforcement rules can raise a verdict independently of the score.

Runtime settings can change thresholds, weights, selected rule modes, network evidence policy, route policy, and request-rate limits. With no active settings overlay, the built-in and startup configuration remains unchanged.

See [How humanymous evaluates a request](docs/concepts/how-gate-sees-a-request.md) for the full flow.

## What Gate enforces

Gate can:

- apply route-specific monitor or enforcement policy;
- refuse request-smuggling shapes and forged internal headers;
- validate fingerprint-bound verdict trust tokens;
- meter requests and apply escalating bans;
- require prior verification before upgrading long-lived connections;
- detect repeated decision probing;
- inject the browser collector into eligible Hypertext Markup Language responses;
- record enforcement and administrative actions in a tamper-evident audit trail;
- require a distinct Approver for protection-reducing runtime settings;
- expose health, readiness, metrics, audit, policy, Settings, and approval surfaces on a separate administrative listener.

Gate does not:

- reproduce Core's complete browser and network evidence;
- see the original client's connection fingerprint behind an intermediary that terminates and recreates encryption;
- provide a production identity provider, independent witness, external immutable archive, or complete appeal system;
- enforce the previously described physical retention tiers;
- automatically route every challenged visitor through a complete, user-solvable Pass flow;
- reliably separate internally consistent real-browser or human-assisted automation from legitimate traffic.

## Quick local run

Requirements:

- Docker Desktop or Docker Engine with Compose
- enough local resources to build the Go images and browser-test image

Build and start Core, Gate, the origin application, and the Ledger:

```powershell
docker compose -f deployments/compose.yaml up -d --build core origin gate
```

Open:

- Core demo: <https://127.0.0.1:8443/demo>
- Gate edge: <https://127.0.0.1:8444/>
- Ledger operator console: <https://127.0.0.1:8445/__hmn/admin/console>
- Detection Observatory: <https://127.0.0.1:8443/playground>

The local stack uses self-signed certificates and deterministic development-only operator credentials. A browser will show a certificate warning. Never use the development credentials for production.

Verify service status:

```powershell
docker compose -f deployments/compose.yaml ps
```

Run the authoritative self-validation suite:

```powershell
pwsh -File scripts/e2e-docker.ps1
```

The self-validation target is always the local deployment you started. Do not aim the bundled profiles at third-party systems.

## Validation catalog

The catalog currently contains 65 entries:

- 63 automated-behavior profiles;
- one synthetic human baseline;
- one explicit coherent-automation boundary case that is expected to receive ALLOW.

The catalog runs against Core. It checks complete profile coverage and fails when a profile errors, skips, or an expected automated profile receives ALLOW.

The synthetic human baseline is not evidence of a population-wide false-positive rate. Current catalog reporting also distinguishes outright denial more strongly than challenge friction, so do not turn its result into a broad user-impact claim.

Run the catalog only through the Docker path described above. Host-loopback runs are useful for profile authoring but do not reproduce the multi-subnet network topology.

## Detection boundary

The design is strongest against direct Hypertext Transfer Protocol clients, common browser drivers, and automation that leaves inconsistent browser or protocol evidence. Confidence falls as automation uses a real browser engine and keeps its browser, network, and interaction evidence coherent.

Internally consistent real-browser automation and workflows using real people cannot be reliably separated from legitimate traffic by client and network signals alone. humanymous mitigates this boundary with request metering, reputation, and opt-in attested-route verification. It does not claim to solve it.

## Deployment topology

Network fingerprints describe the browser only when Core receives the original encrypted connection.

- A transport-layer pass-through load balancer can preserve the client connection.
- A content delivery network, reverse proxy, or application-layer load balancer that terminates and recreates encryption replaces the browser's connection fingerprint with its own.
- The current Gate does not extract Core's full ClientHello and Hypertext Transfer Protocol version 2 evidence even when it terminates encryption.

Read [Supported topologies](docs/reference/supported-topologies.md) before choosing a deployment shape.

## Operator Settings

The Ledger Settings view can propose and apply bounded runtime changes:

- enforcement-rule monitoring;
- challenge and denial thresholds;
- signal-weight multipliers;
- network evidence policy;
- route policy;
- request-rate limits.

The server classifies the change from the actual difference. Protection-increasing changes can apply immediately. Protection-reducing, integrity-affecting, and sensitive-route reductions require a distinct Approver and a bounded expiry.

An empty settings overlay preserves the built-in and startup behavior. Runtime settings never add a second scoring implementation.

## Audit and privacy limits

The audit chain is tamper-evident, not tamper-proof. Signed tree checkpoints and a local witness reveal several classes of modification, but the witness uses the same process and sealed keystore in the reference build.

Identifiers are stored pseudonymously, not anonymously. Anyone holding the keystore and unseal secret can test source values offline without a second approver or an audit record; the reference exposes no re-identification application programming interface.

Cryptographic erasure destroys a session linkage key while retaining the audit record. It is irreversible. The current reference has documented durability, reach, and multi-node limits that an operator must evaluate before relying on it for a data-subject procedure.

Read the [data-processing inventory](docs/reference/data-processing-inventory.md) and [audit verification guide](docs/how-to/verify-audit-log.md) before making privacy or integrity claims.

## Build from source

Requirements:

- Go version declared in `go.mod`
- Node.js for browser tooling and documentation assets
- Docker for authoritative end-to-end validation

Run unit tests:

```powershell
go test ./...
```

Build the browser WebAssembly module:

```powershell
$env:GOOS = "js"
$env:GOARCH = "wasm"
go build ./cmd/wasm/
```

Build the binaries:

```powershell
go build ./cmd/server/
```

```powershell
go build ./cmd/gate/
```

## Documentation paths

- [Documentation home](docs/README.md)
- [Glossary](docs/reference/glossary.md)
- [Integrator start](docs/start-here/integrator.md)
- [Operator start](docs/start-here/operator.md)
- [Evaluator start](docs/start-here/evaluator.md)
- [Data protection start](docs/start-here/compliance-dpo.md)
- [Developer start](docs/start-here/developer.md)
- [Command-line and policy reference](docs/reference/cli-config-policy.md)
- [Ledger tour](docs/how-to/audit-console-tour.md)
- [Self-validation guide](docs/how-to/self-validation-red-team.md)
- [Production responsibilities](docs/reference/production-vs-reference.md)
- [Security disclosure policy](SECURITY.md)

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the repository `AGENTS.md` before changing code or documentation.

Detection behavior is frozen unless a change is explicitly authorized. Any code change must run tests for the touched packages; end-to-end completion authority is Docker.

## License

Apache License 2.0. See [LICENSE](LICENSE), [NOTICE](NOTICE), and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
