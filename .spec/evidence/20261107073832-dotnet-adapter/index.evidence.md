---
id: evidence:root/20261107073832
title: Safe Rust ASP.NET request core unit and library lint evidence
status: active
created: 2026-11-07T07:38:32Z
updated: 2026-11-07T07:39:27Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: 8532f2fd27f395cef5b583620c20cd39809739567128cdc30c89d0735ca38c46, bytes: 1328, store: repo}
---

# Safe Rust request core evidence

Directly inspected the integration chat's completed command handle
exec-b6805700-3564-4e01-bd4e-b2a091e9c933. The actual runner was local Windows
Rust Cargo, jobs1/incremental0, not a managed host. Locked/offline dotnet --lib
tests passed7 with0failed/ignored/measured/filtered. The following locked/offline
six-package --lib Clippy with denied warnings completed exit0. This is library
lint only; it does not establish all-targets/workspace/host qualification.

Earlier E0282/E0283 test compilation and collapsible_if lint failures are retained
in the plan; the raw observation covers the successful follow-up only. A subsequent broader Clippy failed in runtime's
new stream code; it is separate from this completed six-package command.

Source checkpoint c23b504 and current owned source hash are recorded in the
summary. Shared dependency worktree was not immutable and original whole-tree
source/lock hashes were not captured by that run. Post-run hashes are not a
fabricated exact whole-workspace receipt. Full source-bound hosted execution is
still required. No local Kestrel/IPC/process/GC/NativeAOT/Docker pilot occurred.
No NuGet package, C exports, release-qualified host tuple or product e2e success
follows. Test-only finish(ALLOW) exercises lifecycle state, not authority evidence.

Private root-command.json preserves the tool observation and full successful
output; public summary records the exact limits without machine/user inventory.
