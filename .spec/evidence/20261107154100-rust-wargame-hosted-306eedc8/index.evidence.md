---
id: evidence:root/20261107154100
title: Rust wargame hosted306eedc8 strict byte acceptance
status: active
created: 2026-11-07T15:41:00Z
updated: 2026-11-07T15:57:45Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: receipt.json, sha256: 81634effd78eeda3a2aed6afa462ced0eba85af46b6a5a773ea339356087d675, bytes: 3284, store: repo}
  - {path: audit.json, sha256: 9cf7a54350dddc62a9b72b04ca3946a8179f02a2652bcde0eb50b1ad2d6bee9d, bytes: 1759, store: repo}
  - {path: run.json, sha256: 1e073ca3d4bfb2ecfab065557511556bf811761bd94c163a30c8d249de32d8db, bytes: 872, store: repo}
  - {path: delivery-summary.json, sha256: 399011d030b27461ffada5e136659123d02a25a1e02cca62b1f042ed0c1fe48c, bytes: 6926, store: repo}
---

# Rust wargame hosted306eedc8 strict byte acceptance

Actual [run42387564172](https://github.com/modootoday/humanymous/actions/runs/42387564172),
attempt1, source306eedc83801ad153bf8aed68d89a8b1dbb30847, Ubuntu24.04 GitHub hosted.
Preflight128448085478 succeeded; actual Docker product128448838263 failed the
full required gate. Its strict public wargame receipt was recovered intact.

Independent recount:26scheduled=19evaluated+7unsupported, errors0/skipped0/
expectation_mismatches0. Indices0..25 are present exactly once and in order;
unsupported indices0,9,10,16,18,20,22 retain null expectations. All19evaluated
expectations are true. Closed receipt/verifier/totals/case fields match the sole
contract. Schema is humanymous.wargame-network.v2; verifier mode is
owned_process_network, internally_consistent=true, scoped_passed=true,
full_product_qualified=false. The shared strict decoder and complete runtime
profile repair were actually exercised, with no unsupported promotion.

Original report SHA256:
0edae80d11a0ad3d9c4d7baf027ddb1a85f35ea0ad787ad22031461ad4736930.
Lab manifest SHA256:
da210fd18a8ff455a05964c9548a07a6f7a1bfbd32ff516fae58d4ae9a8d8cb4.
Delivery manifest SHA256 is distinct:
98d579bfde1190e1e93f12460a2f4102c2962a47335d9421167e680715415084.
The exact-source lab producer hashes original stdout and its manifest; Delivery
independently hashes both private files before accepting this public projection.
Receipt source matches the externally queried run head. Private original report
bytes were not recovered locally; no independent local original-byte hash is
claimed. Artifact inventory is0. Captured API CLI log text remains private154101.

Actual isolated image IDs are sha256:d4475ceb35a49974137075c48d4000c9ca35ca8054ae7813fbf5a62332d64aa8
and sha256:3abe1f1a279ad3a589e12a2900de4a8cb76a0cac410f108ed3c7bc04f26e5f38.
The existing Rust Compose runner executed; its72assertions passed. The wargame
supervisor is network-none and owns its runtime children. Full product remains
false:9required capabilities unavailable, owner exit2,10outer required cases
blocked. Delivery is component-only/blocked, errors0, source changed=false.
Image IDs are recovered identities, not image contents independently downloaded
or inspected locally. No human/AT or full product qualification follows.

Hosted preflight actually passed attacker17/defender8/Lab26/Delivery90 tests,
all failed/ignored/filtered0, plus actor check, strict all-target Clippy and policy.
Those tests are separate from the26case product denominator. receipt.json is the
recovered allowlisted projection, audit.json is its independent recount, run.json
is external terminal metadata and delivery-summary.json preserves the public
boundary. The original813failure remains evidence083903, not overwritten.
Report migrationc5294b5d and its Windows8pure tests are a separate later source.
