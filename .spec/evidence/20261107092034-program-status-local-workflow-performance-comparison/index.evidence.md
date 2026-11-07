---
id: evidence:root/20261107092034
title: Program status local workflow performance comparison
status: active
created: 2026-11-07T09:20:34Z
updated: 2026-11-07T09:45:20Z
rel:
  depends_on: [sot:root/program-roadmap]
files:
  - {path: local-desktop-act.json, sha256: 803d4f97e8d9db8c1d18e2f22b0aa9cf7e978a96e6de26d5554ff243ad871e02, bytes: 9298, store: repo}
  - {path: github-actions.json, sha256: 2fa8064aec9b4e49ec3b20e8db1a758f88e0f203f153c51f9b046077ab496442, bytes: 9236, store: repo}
  - {path: comparison.json, sha256: c4814e940dba44ed14222b9eb85d3d2a3572533c09b58f236c90442eb472846e, bytes: 2275, store: repo}
  - {path: comparison.md, sha256: 0d179b8b54f111f21575a44c73f21111c6d7cb49bbd02e0a29030a4b4a58b163, bytes: 6544, store: repo}
---

# Program status local workflow performance comparison

## What was measured

Actual local Windows Docker Desktop act execution and genuine GitHub Actions
run 42360069900 executed the same workflow control d610ab4d and workload 8132117.
Both completed successfully, with six matched valid records and unchanged
source. Image digest, Rust/cargo versions, lock hash, UID, cgroup CPU/memory/swap
limits and GNU time package version match. The attached machine reports retain
each workload's time, counts and resource scope; comparison.md explains limits.

## How to read it

Initial and same-job repeat are different cache states. Earlier suites affect
later suites, and host CPU topology/kernel/network/storage differ. GNU max RSS
is not a whole-container peak; cgroup context was collected before workloads.
The local engine ID is retained privately, not published in this projection.
Exact raw CLI/ACT logs, outer controller and earlier native Windows/WSL attempts
remain in evidence:root/20261106194410. Neither these passes nor the matched
environment establish full-product qualification or a universal speedup.
