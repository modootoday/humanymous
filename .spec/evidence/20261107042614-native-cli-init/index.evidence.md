---
id: evidence:root/20261107042614
title: Native init CLI implementation and bounded manifest publication
status: active
created: 2026-11-07T04:26:14Z
updated: 2026-11-07T04:35:44Z
rel:
  depends_on: [sot:root/native-embedding, plan:root/20261107042614]
files:
  - {path: results.json, sha256: dd327a290bc19451b59e4a390c740484265710a50c7ecd89906e6f4ed683731d, bytes: 8468, store: repo}
  - {path: manifest-example.json, sha256: 44a663b163506e5a57741c4821eca637215d355cc461aa891a8d7fbf2194c8ac, bytes: 1303, store: repo}
---

# Native init CLI implementation and bounded manifest publication

## What was measured

Rust humanymous-native-cli / binary humanymous-native init on Windows x86_64.
Final unit11 and actual CLI process6 pass, zero failed/ignored. Locked release
build, fmt and warning-denied Clippy pass. Actual release pilot planned all6
framework families with zero workspace files; explicit write created1 JSON;
repeat write refused with unchanged original hash; option-swallowing conflict
returned2 and an empty workspace. Manifest remains not_implemented/qualified=false.

## How to read it

results.json binds the actual release binary, source files and shared Cargo tuple.
manifest-example.json is the exact synthetic generated artifact, without paths,
credentials or host inventory. Raw records and executable are private in
.spec.local/root/evidence/20261107042615-native-cli. Failed metadata-only handle
prototype is retained separately; it is not counted as a final passing run.
Static junction/actual rename denial, concurrent no-clobber, partial-write cleanup
and postpublication real sharing-failure receipts are native component evidence.

## Limits

No SDK or framework adapter is implemented or qualified. Requirements in a JSON
file are not observed sandbox policy. Non-Windows behavior, network/SUBST mappings,
power-loss, hostile same-user mutation and full Docker product e2e are unverified.
Writes require trusted non-mutating workspace entries; temp cleanup after handle
close is path-based. Atomic no-clobber publication is not crash durability or a
host-security proof. No foreign templates, installer/hooks, network/profile access,
scoring change or sandbox relaxation. Read SDK-15..18 and plan:root/20261107042614.


Canonical-byte correction: attached results.json is explicitly LF under
.gitattributes; its declared hash/length cover clean-checkout Git bytes. Existing
validation-worktree source hashes remain intact, with canonical Git source hashes
recorded separately. This fixes text-byte portability only, not runtime evidence.
