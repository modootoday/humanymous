---
id: sot:root/cloud-infrastructure-resolution
title: Self-hosted first resolution of product blockers with GitHub and Cloudflare profiles
status: active
created: 2026-11-17T09:00:00Z
updated: 2026-11-17T10:30:00Z
verify: manual
rel:
  depends_on: [sot:root/container-test-environments, sot:root/cross-os-runners, sot:root/delivery, sot:root/audit-key-custody, sot:root/dotnet-adapter, sot:root/common-automation-rules, sot:root/rust-migration]
---

# Self-hosted first resolution of product blockers

Version 0.2.0. Accepted design only. Nothing in this document was deployed,
created or charged. No GitHub ruleset, environment, secret or workflow was
changed. No Cloudflare account was inspected or used. Version 0.2.0 applies
the owner's 2026-11-17 direction (self-implementation first) and an
adversarial critique of 0.1.0 (§10).

## 1. Problem

The Rust product lab reports nine `required_unavailable` entries and sets
`full_product_qualified:false` and `release_ready:false`. All three are
hard-coded literals in `test/lab/runner/src/orchestration.rs`. The delivery
`docker` and `browser` pilots (`scripts/ci/delivery/src/pilot.rs`) also
require a `human-control` case. Several owner-only items sit in the
consolidated `needs-owner` Issue.

The owner asked for a canonical design for every blocker that GitHub or
Cloudflare infrastructure can resolve. They then directed that storage and
similar core functions are implemented by the project itself, and that
Cloudflare applies only when an operator adopts a cloud deployment. This SoT
decides, per blocker, the resolving venue, the evidence label and the
residue left to the owner or a human.

## 2. Measured inputs (2026-11-17)

| Input | Measurement | Source |
| --- | --- | --- |
| Repository visibility | public | `gh api repos/modootoday/humanymous` |
| Standard GitHub-hosted runners | free for public repositories; hosted Windows is elevated/UAC-disabled | [[sot:root/cross-os-runners]] R1-R3 |
| Product e2e venue | `product_e2e.rs` refuses `GITHUB_ACTIONS=true` and Windows; only the named cloud-container venue is implemented | `test/lab/runner/src/product_e2e.rs` |
| Existing hosted-venue work | `h2-hosted-venue-validation.yml`; hosted checks in `go_product.rs` and `components.rs` record `RUNNER_ENVIRONMENT` but treat it as unauthenticated | repository |
| Existing witness | `CheckpointWitness` trait (`public_key`, size-based `last_size`, `validate_local_head`, `counter_sign(checkpoint, proof)`) and the local `DurableWitness` | `crates/sdk/humanymous-runtime/src/audit/checkpoint/durable.rs` |
| Generated glue rule | "No generated foreign glue or installer hooks" | [[sot:root/rust-migration]] |
| Cloudflare Durable Objects | SQLite-backed objects are on Workers Free; over a free limit the operation fails | Cloudflare docs, durable-objects pricing and limits |
| Ruleset | `codex-rust-protected` requires `validate / merge-ready`; no bypass actor | consolidated `needs-owner` Issue |
| docs.github.com | not reachable from this cloud session; GitHub claims below come from existing SoT sources and must be rechecked at implementation | session network policy |

## 3. Vocabulary

| ID | Meaning |
| --- | --- |
| venue `lab` | Isolated Docker topology with keys generated inside it. Evidence type `rust-product-lab`. Free and self-hosted. Qualification authority for the product. |
| venue `gh-hosted` | A standard GitHub-hosted fresh VM in this public repository, confirmed through the Actions API (§8). An execution venue for CI, not a product dependency. |
| profile `cf` | An optional Cloudflare deployment profile for operators who adopt Cloudflare. Never required, never qualification authority. Evidence type `provider-fronted` or `provider-remote`. |
| `owner-exec` | The design is complete, but running it needs an owner account, secret, setting, human-gated edit or merge. |
| `human` | Needs a real person or assistive-technology user; no infrastructure substitutes. |
| `lab-target-attestation` | A target attestation signed with a key generated inside the lab topology. It is not "remote" attestation. |

A result from one venue never substitutes for another. The evidence
vocabulary in [[sot:root/container-test-environments]] still applies.

## 4. Decision rules

1. **Self-implemented core.** Storage, witness, transport, attestation and
   deployment are implemented in this repository in Rust and run
   self-hosted. Stores reuse the project's own sealed WAL and checkpoint
   formats. No product path depends on a provider service such as Durable
   Objects storage.
2. **Lab first.** Every blocker that a Docker topology can resolve with
   generated keys is resolved in `lab`. That is the free, self-hosted method
   the Go oracle used.
3. **GitHub-hosted runners are CI venues.** Native OS, .NET and a CI copy of
   the product lab run on standard hosted runners, free because the
   repository is public. Larger runners are out of scope.
4. **Cloudflare only when an operator adopts cloud.** A `cf` profile wraps
   the same self-implemented logic. It must have a self-hosted equivalent
   that qualifies first. A provider limit or error fails closed.
5. **No secret values in the repository or public evidence.** Owner-created
   tokens and secrets are read only by name.
6. **Human-gated and root-owned paths are named.** Changes under
   `.github/workflows/`, `scripts/ci/workflows.json`, `Cargo.lock`,
   `crates/sdk/humanymous-runtime/src/audit/` and root workspace membership
   in `Cargo.toml` go through `needs-owner` PRs or root integration (§7).
7. **Detection freeze holds.** No design here changes a score, weight,
   threshold, signal ID or verdict band.

## 5. Resolution table

| # | Blocker (`required_unavailable` text) | Resolving venue | Design | Residue |
| ---: | --- | --- | --- | --- |
| 1 | full Gate TLS/H2/capture | `lab` | The Gate terminates TLS with a CA generated per topology. The attacker container drives raw ClientHello and H2 frames (`crates/attacker/src/wire`). Capture is taken at the Gate socket. | none |
| 2 | session-bound real browser collector | `lab`; `gh-hosted` for the CI copy | The Chromium container loads the Rust WASM collector from the Gate origin in its own namespace and binds the `hsid` session. | none |
| 3 | Pass recovery | `lab` | Pass challenge, recovery and receipt run end to end against the witness-required Gate. Only the protocol path is exercised. | none |
| 4 | human/keyboard/assistive controls | `lab` for the automated keyboard path only | The automated case proves the keyboard-only, no-motor path reaches Pass without gating on motor richness. Label `automated-a11y-path`, never `human`. | `human`: a real AT user on Windows/macOS. The pilots' `human-control` case stays blocked until the owner decides how it qualifies. |
| 5 | runtime binary authenticated witness transport | `lab` with the self-hosted witness service (§6.1) | The Gate's runtime binary talks to an independent witness service over an authenticated transport. Keys are generated in the topology. | none for the self-hosted path |
| 6 | cross-namespace origin/audit effect product binding | `lab` | The origin and the audit sink sit on separate Docker networks. The Gate binds the origin response and the audit effect to one request ID across namespaces. | none |
| 7 | signed remote target attestation | `lab` gives `lab-target-attestation` only | The target signs with a key generated in the topology; the Gate verifies against the pinned public key. This does not resolve "remote" attestation. Binary provenance (§6.2) is a separate claim. | Remote attestation by a party outside the operator stays open and is listed in the `needs-owner` Issue. |
| 8 | native OS qualification | `gh-hosted` | Pinned hosted labels ([[sot:root/cross-os-runners]]), building on `h2-hosted-venue-validation.yml`. .NET: `actions/setup-dotnet` reads `adapters/dotnet/global.json` (SDK 10.0.403). Retained-launch admission stays refused until its SoT gate is met. | `owner-exec`: workflow and `workflows.json` edits and merge. Hosted elevation is recorded; the standard-user boundary uses the disposable-account actor. |
| 9 | provider deployment | manifests in `lab`; `cf` Tunnel profile is `provider-fronted` | Compose, Kubernetes and systemd manifests ship and are validated in `lab`. A Cloudflare Tunnel to the owned Compose Gate (§6.3) is labelled `provider-fronted`, not a provider deployment. | `owner-exec`: Tunnel account, hostname, Access policy and token. A managed provider deployment stays open. |

Additional blockers outside the nine:

| Blocker | Resolution | Residue |
| --- | --- | --- |
| Product e2e cannot run in GitHub Actions | Add a hosted venue to `product_e2e.rs` that follows the existing hosted checks. Its receipt keeps `authenticated_provenance:false` until §8's API confirmation succeeds. | Workflow job edit: `owner-exec` |
| Merges blocked by the ruleset | No infrastructure workaround. | `owner-exec` |
| crates.io publish | GitHub OIDC trusted publishing, configured by the owner on crates.io | `owner-exec` |

## 6. Designs

### 6.1 Self-hosted witness service

The service is a Rust binary that the operator runs on a different host or
container from the Gate. It serves the existing `CheckpointWitness`
contract over an authenticated transport. Its state is the project's own
append-only sealed log, built from the same parts as `DurableWitness`.

- **Identity.** The witness key is generated on the witness host and never
  leaves it. The witness key directory is distinct from the writer's, as
  [[sot:root/audit-key-custody]] requires. The runtime pins only the public
  key.
- **Stream binding.** A stream is identified by the writer public key plus
  the genesis checkpoint digest, not by a free-form name. A second genesis
  for the same writer key is refused, which prevents starting a fork at
  size 0.
- **Append rule.** `counter_sign(checkpoint, proof)` is accepted only when
  the consistency proof from the stored head to the new checkpoint verifies,
  the size grows, and the writer signature verifies against the pinned
  writer key. A different checkpoint at an already accepted size is refused
  and recorded as equivocation.
- **Idempotent retry.** If the same checkpoint digest is submitted again,
  the stored receipt is returned. A lost response therefore never wedges the
  stream.
- **Rollback detection.** The runtime keeps the latest witnessed size in its
  own sealed state. `validate_local_head` and startup refuse a witness whose
  `last_size` is below that retained size. Witness state is never restored
  backwards.
- **Availability.** Failing closed means witness loss stops the Gate's
  witnessed startup. Checkpoints are batched by size and rate-capped, and
  witness loss is reported as an availability incident, never as success.
- **Client wiring.** The transport client lives outside
  `crates/sdk/humanymous-runtime/src/audit/`. Selecting it at startup needs
  a flag next to `require_external_witness`, which is in the human-gated
  `audit/` tree (§7).
- **Independence statement.** The service is independent of the Gate host,
  not of the operator who runs both. Evidence says exactly that.

**Optional `cf` adapter: blocked by rule.** A Cloudflare Worker built with
`workers-rs` needs `worker-build`, which emits a generated JavaScript shim
and wasm-bindgen glue. [[sot:root/rust-migration]] forbids generated foreign
glue. The adapter is therefore not designed further. It needs an explicit
owner exception, which is listed in the `needs-owner` Issue. If granted, its
witness key must be generated inside the Durable Object as a non-extractable
key, and the evidence must state that the account owner can redeploy the
code and use point-in-time recovery to roll back state. The rollback
detection above still applies.

### 6.2 Binary provenance with GitHub artifact attestations

- Only workflows on `push` to `codex/rust` or a release tag attest. A
  `pull_request` run never attests.
- The image build job requests `id-token: write` and `attestations: write`
  and attests the runtime image digest.
- Verification runs on the host before the lab pins an image digest, because
  lab containers have no egress. It pins `--signer-workflow`, the source ref
  and the OIDC issuer, not only `--repo`.
- The lab normally builds from source; attestation applies to the published
  image consumed by operators and by any lab run that pulls it.
- This proves "built by this workflow at this ref". It never satisfies
  blocker 7.
- Availability for public repositories must be rechecked against
  docs.github.com at implementation.

### 6.3 Cloudflare Tunnel profile (`cf`, `provider-fronted`)

- `cloudflared` runs as a sidecar on the Compose Gate network only. Admin
  and management planes are never published.
- Cloudflare Access (or an equivalent allow list) restricts the hostname to
  the operator's own test clients, and only synthetic data flows. Third-party
  traffic must not enter evidence (defense-only).
- Cloudflare terminates and re-originates TLS, so raw ClientHello and H2
  facts at the Gate are provider facts, not original-client facts.

## 7. Wiring

| Change | Path | Gate |
| --- | --- | --- |
| Lab cases 1-7, 9 manifests | `test/lab/**`, `deployments/lab/**`, `build/rust/**` | routine PR |
| Compute `required_unavailable`, `full_product_qualified` and `release_ready` from per-venue case results instead of literals | `test/lab/runner/src/orchestration.rs` | routine PR |
| Hosted venue in product e2e | `test/lab/runner/src/product_e2e.rs` | routine PR |
| Workflow jobs (native, .NET, product e2e, attestation) | `.github/workflows/*`, `scripts/ci/workflows.json` | `needs-owner` PR |
| Witness service crate | new crate, for example `crates/witness` | crate is routine; workspace membership in root `Cargo.toml` is root integration; new dependencies touch human-gated `Cargo.lock` |
| Witness transport client | new module outside `runtime/src/audit/` | routine PR |
| Startup flag selecting the remote witness | `crates/sdk/humanymous-runtime/src/audit/` | human-gated `needs-owner` PR |

## 8. Verification

- `lab` results come from Docker only, with exact source, images and
  topology (rule 60). A host or loopback run is never authority.
- A `gh-hosted` claim requires the run, attempt and job ID to be confirmed
  through the GitHub Actions API from outside the job, with runner image and
  version. `RUNNER_ENVIRONMENT`, `GITHUB_ACTIONS` and `ACT` are
  self-reported and alone keep `authenticated_provenance:false`.
- `provider-fronted` results name the hostname class and contain no account
  ID or token; they stay separate from lab results.
- A `required_unavailable` entry is removed only when the matching case
  passes in its resolving venue. `full_product_qualified` stays false while
  blocker 4 needs a human and blocker 7 has no remote attestation.

## 9. Owner-only residue

- Merges, workflow and `workflows.json` edits, the `audit/` startup flag,
  `Cargo.lock` and workspace membership.
- The real assistive-technology run (blocker 4) and how `human-control`
  qualifies.
- Remote target attestation by an outside party (blocker 7).
- Any `cf` profile: account, hostname, Access policy, tokens, and an
  exception to the generated-glue rule for a Worker adapter.
- crates.io trusted publishing.

## 10. Critique reflected (0.1.0 → 0.2.0)

- Owner direction: no dependency on provider storage; Cloudflare only for
  cloud adoption (§4 rules 1 and 4).
- `workers-rs` brings generated JavaScript glue, so the Cloudflare witness is
  blocked by rule instead of designed as Rust-only.
- Witness key custody, stream binding, consistency-proof checks,
  equivocation, idempotent retry and rollback detection are specified
  (§6.1).
- Hosted-venue environment variables are self-reported; API confirmation is
  required (§8).
- The hard-coded `required_unavailable` list gets a wiring row (§7).
- Fail-closed availability risk is stated, with batching and rate caps.
- Attestation verification pins workflow, ref and issuer, and never runs on
  pull requests (§6.2).
- Missing human-gated and root-owned edits are listed (§7).
- Tunnel is `provider-fronted`, behind Access, with synthetic data only.
- Lab attestation is labelled `lab-target-attestation`, not remote. The
  unused Browser Run row was removed. Existing hosted-venue work is reused.

## 11. Residual risks

- A witness run by the operator does not protect against a malicious
  operator.
- Hosted runner images, free-tier terms and prices change. Recheck them on
  the execution date and record the observation date.
