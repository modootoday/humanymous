---
id: plan:root/20261108220600-cap-ready-g4-w2-harness
title: Cap-ready G4/W2 harness prep (unlock→Cap burn shortener)
status: active
created: 2026-11-08T13:06:00Z
updated: 2026-11-08T14:13:17Z
rel:
  depends_on:
    - plan:root/20261108220000-post-g4-w2-residual-root-ask
    - plan:root/20261108215600-g4-at-cef-blocked-claim
    - plan:root/20261108210000-r14-h2-serving-consumer
    - sot:root/network-capture
    - sot:root/program-roadmap
  governs: []
---

# Cap-ready G4 / W2 H2 harness prep

Brief: Tip `bb2516af`+ — Unlock C applied + Cap=1 authority-node-ipc EXIT 0
(component only). G4/W2 still Root-blocked. product=0. Cap conserved after this
burn. Never invent `pass_product_clean_room`. Detection freeze.

Private full receipts (gitignored):  
`.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/`

Unlock C candidate TRANSFER (this turn):  
`…/cap-ready/authority-node-ipc/candidate-pack/TRANSFER.md`

## 1. Problem

No Root GRANT / live transfer beyond tip `bb2516af`. Remaining product closers
stay Root-blocked. Cap free alone cannot close product. Candidate-pack
preimages now match live workspace; next Root value is GRANT+transfer (or A/B
unlock) — not Cap spam.

## 2. Decision rules

R-CR1. **No Cap burn this turn.** Cap conserved for structural delta after wake.

R-CR2. **No idle ROOT-ASK re-push.** Ask text already filed; point Root at new
candidate-pack instead of rebroadcasting the same ask alone.

R-CR3. **Cap-ready packs only.** Exact preflight, command preimages, env,
fail-closed expects, and forbidden reburns for W2, G4, and authority Cap.

R-CR4. **Honesty.** Never invent readiness `original_h2` lift, matrix
`pass_product*`, or BrowserReady from component green.

R-CR5. **Candidates out of live paths** until Root allocates + transfers
matching preimages.

## 3. W2 H2 product-binding Cap-ready (summary)

Already landed (not product): R14 serving `5ce40904`; hosted capture
`observe_h2` run 42479169280 = `pass_component_hosted` only; readiness
`original_h2:false`.

Root must allocate: hosted **gateway** H2 product-binding (serve path +
`required.original_h2` + owner integration), distinct from capture five-case;
then release exact paths if Cursor Cap=1 applies the join.

Cap=1 when unlocked (jobs=1, incremental=0, locked):

1. Re-hash gateway/R14 preimages; abort Cap on mismatch.
2. `cargo clippy --locked -p humanymous-runtime --all-targets -- -D warnings`
3. Exact fail-closed: `missing_client_is_challenge_and_missing_planes_are_not_ready`
4. Exact fail-closed: `configuration_refuses_profiles_without_a_tls_h1_consumer`
5. Root-named hosted/clean-room gateway binding pilot (do **not** invent args;
   do **not** reburn `native-component capture`)
6. Readiness probe: lift `original_h2` only if measured; else keep
   `required_unavailable`
7. Honest receipt; product=0 until Root+matrix admit

Forbidden: capture feeder re-dispatch as product; Desktop-as-hosted;
browser/collector/controls/a11y/topology `--require-full` Cap spam.

## 4. G4 CEF/AT Cap-ready (summary)

Status: `blocked_claim`. S9 packets under
`.spec.local/root/evidence/cursor-s9-browser-outer/allocation-packets/` are
read-only preimages (STALE evidence-only; not execution grants).

Root must allocate: interactive CEF/GUI/IME/AT on exact Windows tuple; release
`crates/browser/**` and/or `scripts/ci/cef-pilot/**` and/or lab a11y composition
to `cursor-integration`, **or** admit `browser-ui` with execution allocation.
Preserve a11y non-motor hard rule.

Cap=1 when unlocked:

1. Cap/CEF mutex free; re-hash `crates/browser/native/desktop/pilot-contract.json`
   (S9 prep sha256 `eac4fab4466ae524a84dc3f33ffe70e6ab7537025bba9e7d94a93f052aa8f85a`)
2. Env: interactive desktop; IME/AT installed or explicitly unsupported;
   `HUMANYMOUS_CEF_PILOT_BUNDLE` for process cases
3. If released:  
   `cargo run --manifest-path scripts/ci/cef-pilot/Cargo.toml --locked -- target/cef-native-evidence`
4. If released: GUI/IME/AT observations per S9 `gui-ime-at-pilots.json` list;
   unsupported stays explicit
5. Lab a11y composition **only** if Root names a new clean-room gate — never
   Cap-spam prior a11y/browser-product `--require-full` feeders
6. Honest receipt: component ≠ product; BrowserReady=false unless measured join

## 5. Verification (this turn)

- Private Cap-ready packs under claimed evidence path.
- Unlock C **candidate-pack** authored: INTEGRATION Cap exception, freeze
  allowlist deltas, Dockerfile notes, authority Cap sketch, TRANSFER.md with
  preimage hashes — **not** live-applied.
- This tracked plan mirrors Root review pointers.
- Cap burn=0. product=0. Matrix untouched. No `pass_product_clean_room`.

## 6. Authority Cap-ready (unlock C)

Tip `bb2516af`: `--authority-node-ipc` is `blocked_claim` (EXIT 1 before Docker).
Private pack: `cap-ready/authority-node-ipc/`.
`transfer-probe` reports `unlock_c_preimage_ready=true` after 2026-11-08T14:13:17Z
refresh (`snapshot.rs` preimage `dc743752…`); GRANT.json still missing.

**Candidate pack (reviewable diffs for Root transfer):**  
`cap-ready/authority-node-ipc/candidate-pack/` (`TRANSFER.md`,
`transfer-manifest.json`, `candidate/**`, `patches/`, `notes/`).

**Lab-runner transfer helper (Cursor P0 prep; Cap burn separate):**  
`humanymous-lab-runner transfer-probe --manifest …/transfer-manifest.json`  
verifies live preimages + candidate digests fail-closed.  
`transfer-apply --manifest … --grant GRANT.json` applies only with Root grant
schema `humanymous.cursor-unlock-c-transfer-grant/1`. Never invents
`pass_product_clean_room`.

Exact Root paths (from TRANSFER):

- `deployments/lab/INTEGRATION.md` (host-exec forbid; Cap Docker authorize)
- `deployments/lab/authority/**` (contract + Path A/B notes; freeze inputs)
- `build/rust/authority-lab.Dockerfile` (+ dockerignore)
- `test/lab/runner/src/snapshot.rs` freeze allowlist
- `test/lab/runner/src/orchestration.rs` + new `orchestration/authority.rs`
- optional `adapters/node/src/**`, optional `crates/authority/**` (Path B)

Authority Cap green alone ≠ product.

## 7. Residual / next gap

| If | Then |
| --- | --- |
| Root transfers Unlock C candidate-pack (or A/B unlock) | Cursor Cap=1 executes matching pack; honest receipts |
| Still blocked | Hold Cap free; do not reburn feeders; point at TRANSFER.md |

Exact next Root ask: review `candidate-pack/TRANSFER.md` and allocate **any one**
of A/B/C; then release exact paths for Cap=1. Prefer Cap burn over more docs if
unlocked. Live Codex Root may execute without Cursor inventing ownership.
