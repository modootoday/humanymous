---
id: plan:root/20261117142053
title: "Full-product capabilities, Go port and .NET launch roadmap"
status: draft
created: 2026-11-17T14:20:53Z
updated: 2026-11-19T18:00:00Z
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
as its own `claude/<slug>` pull request, except the remaining Step 3 slices,
which land together through one integration pull request (see "Step 3 delivery
mode"). Detection freeze, Docker-only product e2e, Rust 1.99.1 / edition 2024
and the no-forgery rule apply throughout.

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
`settings_engine` and `pass_scoring` feed or emit scoring inputs. Owner decision
B1 (#75 comment 6979694428) permits exact-parity ports of these rows: weights,
thresholds, hard-rule predicates, verdict bands and signal IDs equal the Go
oracle at 981d25c, nothing is tuned, the frozen `humanymous-core` scoring engine
is reused, and goldens are captured from the pinned oracle and never regenerated.
A parity mismatch is reported, not patched over. No other signal ID, weight or
threshold change is allowed.

The settings slice (overlay, classify, resolve, validate, store_file,
settings_engine) is ported under B1 in
`crates/sdk/humanymous-runtime/src/gateway/settings/overlay`. Its differential
golden `crates/sdk/humanymous-runtime/tests/fixtures/gateway/settings-overlay-go-golden.json`
was captured offline in `mirror.gcr.io/library/golang:1.26.3-alpine@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`
(`--network none`, `GOPROXY=off`, `GOFLAGS=-mod=readonly`, `GOTOOLCHAIN=local`);
the fixture's `provenance` records the command and input and output hashes.
The route-policy slice (`internal/gate/config.go` presets and resolution,
`enforce.go` action mapping, `cmd/gate/routes.go`) is ported under B1 in
`crates/sdk/humanymous-runtime/src/gateway/route_policy` with the golden
`crates/sdk/humanymous-runtime/tests/fixtures/gateway/route-policy-go-golden.json`,
captured the same way. One reported difference is kept: an `Unknown` verdict
always challenges, where Go passed it on safe methods of balanced, monitor and
off routes (owner decisions 2026-11-16 and C1). `pass_scoring` and the
HTTP-level attestation floor (`attestfloor`, `t4_promotion`) remain unported.

### Step 3 delivery mode (owner decision 2026-11-19)

The remaining Step 3 slices no longer land as one pull request each. Every
slice merge rewrote the same shared files (the FM11 ledger totals,
`docs/reviews.json` and the gateway qualification contract), so each merge
forced a resync, a new CI run and a new review of every open slice, and each
CI run waits on the Windows native queue before `validate / quality` and
`validate / msrv` start. From 2026-11-19 the remaining slices land this way:

1. Disjoint lanes are worked in parallel. Each slice stays one or more commits
   with its own rows, Go-captured goldens, `go_test_map` entries and recorded
   deviations.
2. The lanes are merged into one integration branch, `claude/fm11-bulk`. The
   shared files (ledger totals and `by_package`, `docs/reviews.json`, the
   qualification contract) are recomputed once there by the existing
   recount, review-merge and contract-regeneration tools.
3. One pull request carries the integration branch to `codex/rust`. Its body
   has a per-slice table: rows before and after, Go tests mapped, deviations.
4. Independent review runs per slice commit range, in parallel, so review depth
   does not change. Merging still needs every required check green, an
   approving independent review of every slice and no ruleset bypass.
5. The workspace gates (fmt, Clippy with `-D warnings`, `cargo test --locked`)
   pass locally on the integration head before it is pushed, to keep CI reruns
   rare.

Nothing else changes: the detection freeze, the B1 exact-parity terms, the
no-forgery rule and the owner-reserved items stay as written. Pull requests
already reviewed under the per-slice mode (#148, #149) merge as they are.

### Owner decisions delegated 2026-11-19

The owner delegated decisions #6, #8, #9, #10, #11 and #12 and accepted the
review deviations of PRs #150 and #151 in
[#75 comment 7017558784](https://github.com/modootoday/humanymous/issues/75#issuecomment-7017558784).
FM11 lane J (`plan:root/20261119180000`) records them:

- #6: keep the Rust signed one-way node kill; Go's reversible global monitor
  mode is not ported (accepted deviation).
- #8: records bind the signed effective-policy version in place of
  `scope.revision` (`Application::install_signed_policy`).
- #9: `MotorEvidence::GoParity` is never served; it stays `Off`.
- #10: the proxy-VPN and Tor exit feeds stay shadow-only.
- #11: a credential never stands in for request integrity; credential gates
  run after it (stricter than Go).
- #12: model promote and rollback are audited admin actions
  (`gateway::admin_plane::models`, Go-captured golden); training and startup
  never activate a model, and activation changes no weight, threshold or
  signal ID.

Ledger after lane J: 290 `ported_unit`, 13 `partial`, 0 `missing`, 2
`not_applicable`, 9 `oracle_only`.

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
- 2026-11-18: owner decision B1 (#75 comment 6979694428) permits exact-parity
  ports of frozen scoring rows. The settings slice moved eight ledger rows to
  `ported_unit` with a Go-captured differential golden. The overlay is a pure
  library and is not wired into the hosted gateway.
- 2026-11-18: the B1 route-policy slice moved five ledger rows to
  `ported_unit` (enforce, ws5_test, resolve_test, cmd/gate routes and
  routes_test) and mapped R081-R085 and R171-R173. It is a pure library and
  is not wired into the hosted gateway.
- 2026-11-19: the owner approved landing the remaining Step 3 slices through
  one integration pull request with per-slice commits and per-slice
  independent review ("권고에 따라 정본화후 진행해줘", in reply to that
  recommendation; recorded in #75). See "Step 3 delivery mode".
