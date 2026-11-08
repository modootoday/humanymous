---
id: evidence:root/20261108043317
title: Adapter authority namespace admission candidate
status: active
created: 2026-11-08T04:33:17Z
updated: 2026-11-08T04:41:17Z
rel:
  depends_on: [sot:root/adapter-authority]
files:
  - {path: manifest.json, sha256: e00eeb7bd300f255cfc10f73f4a325255f36205116c3aa241634383ff1d63e8b, bytes: 2762, store: repo}
  - {path: CANDIDATE.md, sha256: 53f6c7dfe883c880d1017868b2436750dcb0cfb1a6262e68e585628da8e907eb, bytes: 5290, store: repo}
  - {path: admission_policy.rs, sha256: 55d78d0bb91e91dc22c7a79feb19002b29f50b743f26586d4f4329c5de00eb2e, bytes: 12099, store: repo}
  - {path: admission_policy_tests.rs, sha256: 28a6eb709f72f04521f26af02fa5c416f61ba4482a23929d1d4268c1d00acd57, bytes: 11060, store: repo}
  - {path: authorization.after.rs, sha256: 43349acfc5f59f1f0ae716e73b2398078d30352386cede71e89709f1f5ba91cc, bytes: 7894, store: repo}
  - {path: authorization-tests.after.rs, sha256: 83d186a09b2c76fadb39e367346b48c19d0963692ea18d7f12c1a2928bb4f37e, bytes: 30106, store: repo}
  - {path: observer.after.rs, sha256: 8ffb051fe9324532f727a415779f4b3e0bece14f6d7bf28f2b3baa8cb04e7fc2, bytes: 4808, store: repo}
  - {path: observer-state.after.rs, sha256: 66af32b9c0698996baef68999826f485ee9c531a172330a4179369887a81917a, bytes: 5428, store: repo}
  - {path: observer-tests.after.rs, sha256: aaa6c7355689e1f9c2be8094c519422fd56155fba5c89f07cffd10a5ee0a2efb, bytes: 4660, store: repo}
---

# Adapter authority namespace admission candidate

## What was measured

Source-only Rust candidate, thirteen authored tests, exact preimage-bound manifest.
No compiler, test, fmt, Clippy, runtime or hosted execution; NOT_RUN.
Root isolated source and main application source remain untouched.

## How to read it

Read CANDIDATE.md and manifest.json. DATA policy and uninhabited consuming seam
are not NativeProof, protected namespace/channel/W admission, Ready or release proof.
