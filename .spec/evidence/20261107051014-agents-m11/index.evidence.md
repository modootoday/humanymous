---
id: evidence:root/20261107051014
title: Agents M11
status: active
created: 2026-11-07T05:10:14Z
updated: 2026-11-07T05:29:24Z
files:
  - {path: canon-check-final.json, sha256: cee4f685cd8477ae19dcb041c0cebc8ab3e6812e3e4620e362db5f234cee1f73, bytes: 13084, store: repo}
  - {path: canon-check.json, sha256: a5f48476b072f277ec36435a7cc42ca8e1ba20ecd695422bc0a92a0da59fccaa, bytes: 13083, store: repo}
  - {path: routing-final.json, sha256: c7873f40facd84eb1647418332a638265b4d4419e6961abf4892028f7097483a, bytes: 7804, store: repo}
  - {path: routing.json, sha256: d1053fbf76accfd085c5635d1f5e5e42fff85efd748b95d7778a6ac75fd73316, bytes: 7804, store: repo}
  - {path: source-manifest-final.json, sha256: bd19b542a6a6acbf453b34a91637bbd04d156bc8833f6b23a91c54b41225695c, bytes: 1149, store: repo}
  - {path: source-manifest.json, sha256: e97bbb98eb5b2e55e805a7876f772f9d3ffcc00220b41b623eebbbf766f1de07, bytes: 1174, store: repo}
  - {path: summary.json, sha256: 4e6cd5c2bb01f894c55c8c2d0c4da3a264c3fbdfee31253e5eca21a6cac04eb7, bytes: 1905, store: repo}
  - {path: git-binding.json, sha256: af6c5aae847880d763ac0d340d682d4bb9d956ca8be640ac861b8bc5fcef84f9, bytes: 985, store: repo}
---

# Agents M11

## What was measured

Canonical Rust agent maintenance on Windows x86_64; seven unit and four actual
executable fixture tests, fmt/Clippy, canonical metadata/paths/LF, forty deterministic
routing cases and read-only external skill observation. This is component evidence,
not whole M11, full product or provider compatibility qualification.

Public attached source-manifest-final.json identifies the final validated working Rust source
by per-file SHA-256 plus Cargo/lock hashes and contemporaneous base HEAD. It is
explicitly a dirty-source snapshot, not a claim that base HEAD contains the changes.
canon-check-final.json contains source-relative hashes and observed drift; routing-final.json
is a regex proxy result, never model activation. summary.json records the separate
upstream startup failure and partial independent CLI pilot without raw traces.
Earlier non-final manifests/checks are retained development snapshots. The schema
delta initially failed Clippy; final struct initialization passed. Private
clippy-schema-failed.txt and clippy-final.txt distinguish the two results.

The named modoo.today eval script hash is
7430f009e37a556d1a96f512e31e508569297d35dbf1686766e6686fe48cfd3a.
Its complete plugin/shared package was not supplied by the catalog; actual startup
fails on shared/owned-process.mjs. Private WSL bubblewrap0.9.0-1ubuntu0.3 namespace
probe succeeds. No --isolation none/no-sandbox fallback was used.

Independent Codex0.166.0-alpha.6 Windows read-only pilot: two baseline and one
treatment answers,123450 observed tokens after the100000 future-call stop bound.
The treatment skill-body read was policy-blocked and its body-read activation is
unverified. Automatic historical false quality is interpreted as infrastructure,
not silently rewritten or used for a positive delta. Neither this pilot nor the
40/40 static proxy substitutes for a qualified upstream paired plugin eval.

Raw stdout/stderr/answers/upstream bytes/packages remain private in
evidence:root/20261107051015. The CLI pilot was exploratory before a complete dirty
source snapshot, so it cannot qualify current harness source or any release.
Final unit/process and current source manifests are separately attributed. Earlier
development unit/Clippy logs were reused before the final source snapshot; only
the final-source logs are retained as raw files. No failed CLI/dependency result
was replaced by a success, and no historical source qualification is inferred.

## How to read it

git-binding.json binds the final tested files to canonical implementation commit
9c3b735e759cf060b79a48f71db4ce234a9bd465, separately from the earlier dirty-source
snapshots. Git blob SHA-1 identities are labeled and are not file SHA-256 hashes.

See sot:root/agent-maintenance, plan:root/20261107051013 and
`.agents/INTEGRATION.md` for contracts, current gates, exact root integration
delta and open dependencies. Mirror differences92 and user skills20 are
observations, not implicit waivers or global mutation authority. Root owns common
Cargo/lock, CI/workflow consumers and final manifest-driven retirement.
