---
id: evidence:root/20261108043317
title: Adapter authority namespace admission candidate
status: active
created: 2026-11-08T04:33:17Z
updated: 2026-11-08T04:47:50Z
rel:
  depends_on: [sot:root/adapter-authority]
files:
  - {path: authorization.after.rs, sha256: 43349acfc5f59f1f0ae716e73b2398078d30352386cede71e89709f1f5ba91cc, bytes: 7894, store: repo}
  - {path: observer.after.rs, sha256: 8ffb051fe9324532f727a415779f4b3e0bece14f6d7bf28f2b3baa8cb04e7fc2, bytes: 4808, store: repo}
  - {path: observer-state.after.rs, sha256: 66af32b9c0698996baef68999826f485ee9c531a172330a4179369887a81917a, bytes: 5428, store: repo}
  - {path: observer-tests.after.rs, sha256: aaa6c7355689e1f9c2be8094c519422fd56155fba5c89f07cffd10a5ee0a2efb, bytes: 4660, store: repo}
  - {path: CANDIDATE.md, sha256: 782ab0070735410d749530ed0e9c7f340b811cf98480367cd30d6dd329d6a52f, bytes: 5900, store: repo}
  - {path: admission_policy.rs, sha256: 2148d3c34f2d7b0d12f7684d809845f54141891d7aa9b732c894cafda3bd9998, bytes: 12927, store: repo}
  - {path: admission_policy_tests.rs, sha256: 2762f7319ee3a3836463613bb6c2f0d22478c71f3b2103b3d092b75deec46cab, bytes: 14440, store: repo}
  - {path: manifest.json, sha256: 9ce9667faf6827fad44d03a548dad6c87fce0f8ee3bd6a23883a85e900f2dee1, bytes: 2927, store: repo}
  - {path: authorization-tests.after.rs, sha256: 8117492d753afdc2b60c2091bf9756163a6341f2fe5d755d4496f9f082a6b437, bytes: 33626, store: repo}
---

# Adapter authority namespace admission candidate

## What was measured

Source-only Rust candidate, sixteen authored tests, exact preimage-bound manifest.
No compiler, test, fmt, Clippy, runtime or hosted execution; NOT_RUN.
Root isolated source and main application source remain untouched.

## How to read it

Read CANDIDATE.md and manifest.json. DATA policy and uninhabited consuming seam
are not NativeProof, protected namespace/channel/W admission, Ready or release proof.
