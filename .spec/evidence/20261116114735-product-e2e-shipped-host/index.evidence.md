---
id: evidence:root/20261116114735
title: The shipped CLI runtime host cannot serve the witnessed product topology on Linux (unit 35 increment 5)
status: active
created: 2026-11-16T11:47:35Z
updated: 2026-11-16T11:47:35Z
rel:
  depends_on: [evidence:root/20261116113042, evidence:root/20261116110546, evidence:root/20261116103304, plan:root/20261116060213, sot:root/runtime-application, sot:root/cli-installation, sot:root/container-test-environments]
files:
  - {path: receipt.json, sha256: 316ee6df94cdaad13ce78712f25b30f7b12f04eb8f1cbee56941c585b11df64a, bytes: 3016, store: repo}
---

# The shipped CLI runtime host cannot serve the witnessed product topology on Linux (unit 35 increment 5)

## What this increment tried

Increments 2 to 4 left one gap: the witnessed and browser Gates are the
runtime library composed by a lab supervisor, not the shipped product
([[evidence:root/20261116113042]], [[evidence:root/20261116110546]]). The
legacy `humanymous-runtime` gateway binary calls only `ServiceHost::bind`.
The shipped CLI host (`humanymous runtime-host`) does call
`ServiceHost::bind_with_witness` when it is handed an `ExternalWitness`. This
increment checked whether the witnessed Docker topology could run that CLI
host instead of the lab supervisor.

It cannot, for three source-proven reasons. No Docker topology was changed or
run. This bundle records the source findings and one native process
observation of the shipped host, not a product e2e run.

## Findings

1. **The Unix host did not compile with `runtime-service`.** At the parent
   tip `63a83ff`, `cargo build -p humanymous-cli --features runtime-service`
   failed with E0107 at `crates/cli/src/runtime_host/platform.rs:55`. The
   readiness publisher was passed as `output::<HostReadiness>`, but `output`
   takes `impl Serialize`. Commit `d649b9a` passes a typed closure instead.
   Behaviour is unchanged. Before that commit, `--all-features` builds of
   `humanymous-cli` also failed on Linux.
2. **The Unix host has no witness input.** The foreground path calls the
   shared `run` with `witness = None` (`platform.rs:49-56`). No CLI argument,
   plan field or broker supplies an `ExternalWitness` on Unix. In the governing
   design, the witness is the observer O's subordinate child W, and the broker
   owns peer authentication and deadlines (`sot:root/runtime-application`,
   "Inject trusted CheckpointWitness …" and the R1/R3 observer topology). R1
   native backend and R3 typed activation/API review are recorded there as
   pending, so a lab-only witness transport into the shipped host would be a
   new design, not a harness change.
3. **The Unix host cannot obtain its unseal credential.** The host reads the
   audit unseal passphrase only through `native_security::secret`. On
   non-Windows that returns `Unsupported`
   (`crates/sdk/humanymous-runtime/src/native_security/secret/mod.rs:34-41`).
   `sot:root/cli-installation` states "Non-Windows providers currently return
   Unsupported." A plaintext passphrase in the configuration is refused.
   Supplying one would need a new Linux secret provider and audit unseal
   material, which is owner (needs-human) work.

## Native observation

One build and two runs at source commit
`d649b9a5dfafa7ada26082a80ecb336cab5c838d` (clean tree), as root, on the
cloud session's Linux x86_64 host. This was not Docker.

- **Artifact**: `cargo build --locked -p humanymous-cli --features
  runtime-service --bin humanymous` (dev profile, rustc 1.99.1). Binary
  SHA-256 `52f9664b8ebe676f73eaac5351dd9e63e68010012a84e8de9c6324f9077bff98`.
- **Input**: a digest-pinned `humanymous.runtime-host.v1` foreground plan
  with no observer, and a non-secret runtime configuration with an empty
  passphrase and `allow_new=true`. Files were 0600 in 0700 directories.

| Case | Exit | Failure envelope | stderr | Audit/key directory created |
|---|---|---|---|---|
| `require_external_witness=true` | 1 | `stopped=false`, `product_qualified=false` | `refused: required independent witness provider unavailable` | no |
| `require_external_witness=false` | 1 | `stopped=false`, `product_qualified=false` | `refused: protected service credential unavailable` | no |

Both refusals came before custody was opened and before any listener or
readiness was published. That is the documented fail-closed order.

## Live readiness (task not done)

The CLI host's diagnostic listener answers the plan's fixed `ready_response`.
`ServiceHost::epoch_health` exists, but no governing clause lets the CLI
diagnostic follow it.

- `sot:root/cli-installation` defines `ready_response` as an "exact
  diagnostic response" that `service-doctor` compares.
- `sot:root/runtime-application` makes live readiness part of the typed epoch
  admission gate ("Revocation … disables future live readiness"). It says
  existing Ready publication "cannot be reused as application readiness", and
  it records R3 typed activation/API review as pending.

Changing the diagnostic answer would therefore need a SoT delta. It was not
implemented. It also could not be exercised in Docker while findings 2 and 3
stand.

## What is still absent, and why

- **Shipped-binary witness and live readiness**: still absent. This needs:
  - a Linux service secret provider;
  - a typed witness/broker input for the Unix host;
  - a SoT delta for live diagnostic readiness.
- **Everything else**: unchanged from [[evidence:root/20261116113042]]. That
  covers authority, browser path provenance, DNS reachability, report replay,
  origin isolation and clean room.

## Sanitization

The receipt was assembled by hand from the observed stdout, stderr and exit
codes. Host paths are replaced by `<private-plan>`. It contains no
passphrase, key, token or audit material, because none was created.

## Cleanup

The Docker daemon was not started. No images, containers or build cache were
created. The scratch plan and configuration files were deleted after the
observation.

## How to read it

- `status` is `blocked_before_docker`. `pass_product_clean_room`,
  `product_complete` and `clean_room` are false.
- The two refusals are observations of the shipped host. They do not change
  any increment 4 product verdict.
