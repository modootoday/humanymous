# CONSOLIDATED-ROOT-ASK — Cap=0 tipbind readiness (tip 905f2e22)

Observed: 2026-11-09T22:54:56Z · HEAD `905f2e22aea4796978ccf9a92b5b01d9520b6394` · session `cursor-integrator` · key `cursor`  
Decision: **B** · Cap burn this turn: **0** · Cap free: **false**  
product: **0** · `pass_product_clean_room=0` · BrowserReady=**false** · `n06_owner_join=false`  
Unlock: A=**true** · B=**true** · C=**false** (6 digest mismatches)  
Detection freeze. Never invent Root AUTH/AT tokens, Cap free, BrowserReady, or `pass_product_clean_room`.

## Verdict

**Root_blocked on critical path.** Cap0 residual tip-drift closed this turn
(matrix `ef8f1eea` → LIVE `905f2e22`; `tip_bind.ok=true` ≠ product).  
No Cap1 burn (Cap free=false). No Cap twin reburn (G2/G5/CR/G6/RB/PM · 73–80).

## Exact Root handoffs (mutually sufficient for Cap0 apply path)

### 1) AT-HANDOFF — BrowserReady stack (ranks 1–6)

| Field | Value |
| --- | --- |
| Schema | `humanymous.root-interactive-at-windows-tuple/1` |
| Place file | `.spec.local/root/evidence/root-interactive-at-windows-tuple/AT-HANDOFF.json` |
| Required | `browser_ready_authorized=true`; `product=0`; `pass_product_clean_room=0` |
| Cap-ready template | `.spec.local/root/evidence/20261110074900-cursor-cap0-tipbind-readiness/cap-ready/at-handoff-apply/candidate-pack/AT-HANDOFF.TEMPLATE.json` |
| After land | Cursor Cap0 `handoff-apply --kind at` → measured BrowserReady join (still product=0 until matrix admits) |

### 2) AUTH-HANDOFF — N06 owner-join (ranks 7–8)

| Field | Value |
| --- | --- |
| Schema | `humanymous.root-n06-owner-join-auth-handoff/1` |
| Place file | `.spec.local/root/evidence/root-n06-owner-join-auth-handoff/AUTH-HANDOFF.json` |
| Required | `n06_owner_join_authorized=true`; `product=0`; `pass_product_clean_room=0` |
| Cap-ready template | `.spec.local/root/evidence/20261110074900-cursor-cap0-tipbind-readiness/cap-ready/auth-handoff-apply/candidate-pack/AUTH-HANDOFF.TEMPLATE.json` |
| After land | Cursor Cap0 N06 honesty/join (never invent `n06_owner_join=true` from handoff alone) |

### 3) Unlock C digest rematch (rank 10 / 52) — exact live digests

Manifest: `.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/authority-node-ipc/candidate-pack/transfer-manifest.json`

| destination | live_kind | set expectedSha256 to LIVE |
| --- | --- | --- |
| `build/rust/authority-lab.Dockerfile` | preimage_mismatch | `1a926ad99874885b84b1bbebaec0056fdd92ab1ff30989ae1741c9e6cb427cc3` |
| `deployments/lab/INTEGRATION.md` | preimage_mismatch | `b7fa7dc2fad1e3b24f247dea657499e4a4dcac04f45d66e948665112bb001315` |
| `deployments/lab/authority/cap-pilot-contract.json` | unexpected_live | `07acf975acdf7732083d6569ae13a43716c274477600204201991493f7b7a42f` |
| `deployments/lab/authority/CAP-PILOT.md` | unexpected_live | `097747547f88cf39e8dfc609f7e7e6bafe77176bd3c28ce41589857c1fc1a570` |
| `test/lab/runner/src/snapshot.rs` | preimage_mismatch | `04fafd385cb067bbb75fdaa332c8f570429ce0cae5e74eb487b9dc8d3845d00e` |
| `test/lab/runner/src/orchestration/authority.rs` | unexpected_live | `bfadc77fbeb4f0912dbd247feca947e6dfba9f188b3ba2c99449ee07b2f4d1da` |

Full JSON: `.spec.local/root/evidence/20261110074900-cursor-cap0-tipbind-readiness/unlock-c-rematch/ROOT-REMATCH-DIGESTS.json`

### 4) Cap realloc — NEW non-twin only (rank 9)

Cap free=**false**. Spent feeders forbidden: G2/G5/CR/G6/RB/PM.  
Checklist: `.spec.local/root/evidence/20261110074900-cursor-cap0-tipbind-readiness/rank9-cap-realloc/CAP-REALLOC-ELIGIBILITY.json`  
Need NEW Cap-ready GRANT→TRANSFER matching LIVE preimages; then Cap=1 once `--clean-room-product` / product-matrix toward `pass_product_clean_room`.

## Cursor Cap0 executed this turn

- Fresh inspect @ `905f2e22`: Cap free=false; AT/AUTH absent; CEF durable `cap_day_bound`
- Matrix tip-bind `ef8f1eea`→`905f2e22` (rank 13 Cap0 residual the reaudit missed vs postpush tip)
- Unlock C digest rematch + Root rematch packet (rank 10)
- AT/AUTH Cap-ready templates + ROOT-ONE-STEP wakes tip-bound
- Cap realloc eligibility checklist + Cap1 HOLD landers (no Cap1 burn)
- CONSOLIDATED-ROOT-ASK tip-bound

## Exact next closer

Root AT-HANDOFF + AUTH-HANDOFF → Cap0 apply BrowserReady/N06 → NEW non-twin Cap realloc → Cap free=true → Cap=1 product path toward `pass_product_clean_room` + full matrix + red-blue-validate.

goal_complete=**false**