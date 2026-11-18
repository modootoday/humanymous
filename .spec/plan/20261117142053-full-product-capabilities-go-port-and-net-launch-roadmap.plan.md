---
id: plan:root/20261117142053
title: "Full-product capabilities, Go port and .NET launch roadmap"
status: draft
created: 2026-11-17T14:20:53Z
updated: 2026-11-18T03:57:40Z
rel:
  depends_on: [plan:root/20261116060213]
---

# Full-product capabilities, Go port and .NET launch roadmap

## Goal

On 2026-11-17 the owner chose, for the open items in issue #65:

- build the nine missing full-product capabilities as roadmap work rather than
  signing exceptions;
- port the unported legacy Go rows to Rust before the Go tree is removed;
- finish the .NET launch path now rather than deferring it.

`codex/rust` is complete only when `delivery ready` passes on every axis at HEAD
and `validate / merge-ready` is green. This plan orders the work. Each step lands
as its own `claude/<slug>` pull request. Detection freeze, Docker-only product
e2e, Rust 1.99.1 / edition 2024 and the no-forgery rule apply throughout.

## Step 0: merge deadlock (owner)

Every pending pull request fails the required check `validate / merge-ready`. That
check is red because of base-branch defects which those same pull requests fix:

| Defect | Fixed by |
| --- | --- |
| runtime-lib inventory mismatch | #41 |
| retired Go images | #59 |
| `publish=false` on all crates | #61 |
| missing Node qualification receipt | #64 |
| documentation review inventory | #63 |
| Go advisories | #58, #44 |

The `codex-rust-protected` ruleset has no bypass actors, so no pull request can
merge. Until the full product qualifies (Step 2), `merge-ready` cannot be green at
all. The owner must either add a bypass actor (for example the repository admin
role in pull-request mode) or temporarily require only the per-axis jobs a pull
request touches. After that, merge order is #41, #40, #42, #58, #44, #59, #61, #63,
#64, #60. Agents rebase and re-run Validate after each merge.

## Step 1: .NET launch path

Accounting rows 24/72/73 are the dotnet hosted/package ignored tests. They need a
real launch executor (`actor/prebuilt.rs`) and a pilot declaration in
`scripts/ci/native-pilots.json`, mapped by name like the other pilots. Owner input:
the launch API shape (hosted process or package entry point) recorded in #65.

## Step 2: full-product capabilities

Order follows dependency. Each capability turns one `required_unavailable` entry in
`test/lab/runner/src/orchestration.rs` into an evaluated product case. The flag
`full_product_qualified` becomes true only when every case passes in Docker.

| # | Capability | Product cases | Needs |
| --- | --- | --- | --- |
| 2a | full Gate TLS/H2/capture | gate, raw-tls-h2 | certificate source (Step 3 B02) |
| 2b | runtime binary authenticated witness transport | attack-catalog, swarm | — |
| 2c | cross-namespace origin/audit effect binding | origin-isolation, audit-restart | — (Go `internal/gate`, `internal/audit`) |
| 2d | session-bound real browser collector | collector-parity, missing-signals-closed | browser image |
| 2e | Pass recovery | pass | — |
| 2f | human, keyboard and assistive controls | human-control, challenge-keyboard, challenge-assistive | Windows AT venue for the native part |
| 2g | native OS qualification | native axis | Step 1 |
| 2h | signed remote target attestation | deployment | beyond Go parity; lab key generated inside the Docker topology |
| 2i | provider deployment | deployment, documentation-examples | Go parity is compose/k8s/systemd manifests; a live provider is beyond Go |

### 2i follow-ups (owner decision C8, #75)

The owner added two items to the roadmap on 2026-11-18. Both close the two known
gaps recorded in #94: the shipped manifests cannot start a runtime unattended.
Neither changes detection, and neither adds a k8s cluster or systemd PID 1 to the
lab. "Provider deployment" stays the Go-parity manifests; k8s and systemd keep
their static tests.

| # | Item | Scope | Acceptance |
| --- | --- | --- | --- |
| 2j | stdin config injection | A shipped, unattended way for compose and k8s to deliver the one private JSON configuration document on the runtime's standard input (systemd already uses `StandardInput=file:`). The runtime keeps refusing argv, and it never reads the configuration or the passphrase from the environment or a public mount. A SoT delta for `runtime-application` comes first and names the mechanism and its fail-closed cases. | `deployments/compose.release.yaml` and `deployments/k8s/` start the runtime with no interactive `run -T … <` or `kubectl attach -i` step. The Docker `deployment` and `documentation-examples` cases run that path. Missing, empty, truncated or oversized input fails closed before bind. `deployment_manifests.rs` keeps the no-argv and no-secret-env checks. |
| 2k | custody creation command | A shipped Rust CLI operation, not an HTTP endpoint, per the `runtime-application` and `audit-key-custody` SoTs. It creates `keys.sealed` once in an already provisioned owner-only directory, using the frozen envelope and the existing vault API, and refuses existing, pending, symlinked or permission-widened custody. The passphrase arrives on a private pipe, never in argv, the environment or logs. It replaces the generation code that today exists only in the lab supervisor. | Unit tests cover create-once, refuse-existing, refuse-pending, wrong owner or mode, and passphrase-policy refusal. The Docker `deployment` case provisions custody with the shipped command instead of the lab actor, and the runtime then starts from it. The `deployments/` READMEs document the command and drop the "no shipped command" gap. |

Red-blue product (RB) and clean-room product matrix (PM) runs need owner-authored
grants, per `transfer_applicator.rs`. Agents never author grants. A grant is a
governance step for a run, not a missing capability: the implementation work above
proceeds without one.

### Cross-check against the measured Go implementation

Measured at oracle 981d25c on 2026-11-17:

- **No paid or owner-held resource.** Go TLS used `autocert` with free Let's
  Encrypt (TLS-ALPN-01, `DirCache`, optional email, `AcceptTOS`, host whitelist);
  `-acme-directory` selected staging or a private CA. BYO pairs used
  `tls.LoadX509KeyPair`. Self-signed was the default when nothing was set. The
  Rust issuer (#68) follows the same path without a production self-signed
  fallback.
- **Automatable at Go parity:** 2a (`cmd/server` tlsconn, h1capture, h2capture),
  2b (`internal/audit` witness, merkle, proofs), 2c (`internal/gate`), 2d (`web/`
  collector), 2e (`pass_handler`, store, proof), and 2g on GitHub-hosted runners.
- **Beyond Go:** 2h. Go `internal/attestation` was a local HMAC demo. Signed remote
  attestation is new Rust scope; a lab key generated in the topology suffices.
  2i: Go shipped only deployment manifests, which are ported as manifests; the
  container SoT keeps a live provider out of the Docker lab.
- **Needs a desktop:** only the native Windows assistive-technology part of 2f.

## Step 3: legacy Go port (FM11 ledger)

`.agents/retirement/legacy-go-row-review.json` has 95 `partial` and 52 `missing`
rows. Port them by plan slice:

| Slice | Rows | Notes |
| --- | --- | --- |
| B02 certificate source (ACME, BYO, dev self-sign) | cmd/server, cmd/gate tls/acme/cert | #67 source, #68 ACME issuer; dev self-sign and a live directory run remain; unblocks 2a |
| B23 Redis client, ledger, rate limit | internal/redis, internal/gate redis* | |
| B24 Redis/ClickHouse audit projection | internal/audit sinks | |
| B11 playground, livefeed, launch parity | cmd/server playground/launch, internal/livefeed | lab-only replacement |
| B21 admin console; B15 loader injection; B05 demo | internal/gate console/inject, cmd/server demo | |
| resource gate, embed/media | internal/resource | emits existing signal IDs only |
| watermark LSB, recovery, issuance | internal/watermark | |
| mlcorrect-train promotion gate | cmd/mlcorrect-train | SoT model-training-correction |
| attacker clients | cmd/redteam h2raw, cmd/tlsparrot | defense-only, this deployment |
| remaining `partial` rows | internal/gate, cmd/server, others | close the recorded gaps |

**Detection freeze:** the settings overlay, classify and resolve rows,
`settings_engine` and `pass_scoring` feed or emit scoring inputs. They are ported
only after the owner explicitly unfreezes or approves an exact-parity port. No
signal ID, weight or threshold changes otherwise.

When every row is `ported_unit` or explicitly oracle-only, remove the Go sources,
`go.mod` and `go.sum` in a needs-human pull request, keeping the oracle pinned at
981d25c.

## Step 4: final validation

Docker red-blue-validate (state source, image, topology and runner), workspace
`cargo test --locked`, fmt and Clippy, wasm32 collector check, an actual browser
pilot, and `delivery ready` on every axis. Summarise in #65 and update #22.

## Record

- 2026-11-17: the owner chose roadmap implementation, porting Go before removal,
  and handling .NET now. A merge attempt on #41 was refused: "Required status
  check validate / merge-ready is failing". The ruleset has no bypass actors.
- 2026-11-17: a cross-check against the measured Go implementation found no paid or
  owner-held resource in Go's TLS path. The owner G6 grant, owner signing key and
  owner provider account were removed as capability prerequisites.
- 2026-11-18: owner decision C8 (#75 comment 6968092014) added 2j (stdin config
  injection) and 2k (custody creation command). The public runtime image now
  ships a 65532-owned `/var/lib/humanymous` (mode 0700), and the stale
  `nginx-adapter → docker/deployment` documentation-review binding moved to
  `docker/nginx`. No k8s cluster or systemd PID 1 in the lab.
