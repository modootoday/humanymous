# Authority Cap pilot notes (Unlock C candidate)

## Why Cap is blocked today

1. `deployments/lab/INTEGRATION.md` says the runner never executes `authority/pilot.mjs`.
2. Cap CLI `--authority-node-ipc` currently fails closed as `blocked_claim` before Docker
   (`test/lab/runner/src/orchestration.rs`).
3. Freeze allowlist excludes `pilot.mjs`, `authority-lab.Dockerfile`, and `adapters/node/src/**`,
   so even a Docker Cap path cannot immutable-build the existing authority-lab image.

## SoT-aligned unlock (minimal Path A)

- Amend INTEGRATION: forbid **host** execution; authorize Cap CLI Docker coordination of
  `compose.authority.yaml` from freeze-allowlisted inputs.
- Expand `snapshot.rs` allowlist for Dockerfile(+dockerignore), authority package files,
  `pilot.mjs` (COPY input only), and `adapters/node/src/*.{cjs,mjs,cts,mts,json}`.
- Wire Cap CLI to build/run `authority-pilot` via Docker only; record component receipt;
  never invent `pass_product_clean_room`.

## Preferred retirement (Path B)

- Author Rust Cap pilot that replaces Node `pilot.mjs` ENTRYPOINT.
- Optional ownership under `crates/authority/**` or runtime package binary.
- Drop `.mjs` from Cap freeze once Rust pilot is Cap-bound.

## Not claimed by this candidate

- Live writes to Root-owned released paths (this pack is evidence-only until transfer).
- Product cell promotion / Cap spam / detection thaw.
