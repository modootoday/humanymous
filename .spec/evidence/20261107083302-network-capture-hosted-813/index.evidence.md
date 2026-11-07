---
id: evidence:root/20261107083302
title: Exact 813 hosted compile and incomplete capture execution scope
status: active
created: 2026-11-07T08:33:02Z
updated: 2026-11-07T08:33:02Z
files:
  - {path: receipt.json, sha256: edf6e8d7ababba62007de7cc330670ec4149e35117a07fea8468bede75d2d1de, bytes: 3511, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107072531]
---

# Hosted job observation for exact813

Run42355168729 targets8132117e7bf1445939d7dee9066fcf45ccf0915b. At this
observation the run is still active; only the Linux native and Docker jobs below
are terminal. Policy passed; macOS/Windows/browser outcomes are not claimed.
The original a26d startup failure remains separately recorded and is unchanged.

Linux job128351926093 built the default-feature workspace and visibly compiled
humanymous-capture using Rust1.99.1/x86_64-unknown-linux-gnu. Its actual native
actor then failed:15of88required harnesses emitted,51passed tests,0failed tests,
4raw ignored,1error and2zero-test harnesses. The last emitted harness was
host_process with2passed/3ignored and validated=false. No capture harness exists
in that executed report. Therefore the compile is observed, including the
default test target's Send/Sync bounds, but the new22capture unit executions are
not qualified by this report. Neither hosted-feature compilation nor the five
ignored capture pilots ran through a configured job in this candidate.

Docker job128351926068 failed in the actual lab with diagnostics
actual-owned-wargame/assertion_failed and execution/wargame_process. Its receipt
has full_product_qualified=false, component-only qualification and9required
unavailable capabilities. Exact image:
sha256:6e090be2e6c18b17eac7ad3ac0a25ac7b627779ef1940c21fa7b572d045b9a9f.
The source manifest hash and source_changed_during_execution=false are preserved
in the attachment. No capture/runtime body/stream product qualification follows.

Original completed-job logs are retained privately and hashed in the receipt.
The gh run-view log command refused an active overall run; the completed-job log
API was then used. Its initial ANSI terminal-render guard was handled by saving
raw output directly to private files and stripping escapes only in derived
analysis. One stderr diagnostic interleaved inside the printed Docker JSON was
excluded from the JSON parse only and preserved verbatim in the receipt; raw
logs were not edited. It is an explicit failure, not suppressed evidence.

Root was notified of both failures and the absent capture harness/pilot receipts.
No source, push, dispatch or remote job cancellation was performed here.

- https://github.com/modootoday/humanymous/actions/runs/42355168729/job/128351926093
- https://github.com/modootoday/humanymous/actions/runs/42355168729/job/128351926068
