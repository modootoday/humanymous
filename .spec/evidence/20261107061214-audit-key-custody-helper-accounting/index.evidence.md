---
id: evidence:root/20261107061214
title: Audit key custody helper accounting
status: active
created: 2026-11-07T06:12:14Z
updated: 2026-11-07T06:14:39Z
rel:
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: contract.json, sha256: 8d61b8fdacd268845eaaac62ffcabd837eb5e8834512a840e6cd38a50df7b95d, bytes: 3013, store: repo}
---

# Audit key custody helper accounting

## Historical scope

Original governing path: `crates/audit/tests/durable_witness.rs` in the former
humanymous-audit package. Evidence/source-recording commit:
5528d74fe397c536aa9742f03b05f54f4cfd56e4. The original working-source hash and
post-commit equality distinctions below and in contract.json remain unchanged.
After Runtime relocation64f74c7a this retained evidence has no active path
governance; it does not qualify the moved Runtime implementation or rerun tests.

## What was measured

Test-only correction of inherited libtest child output. Actual Windows targeted
suite durable_witness12/key_backups8 and Linux local Docker11/7 passed. Each has
one raw ignored helper, one outer summary and an actually passing required parent;
child stdout/stderr are bounded and privately captured/asserted. Ignored helpers
are excluded from success counts. Targeted Clippy and formatting passed.

contract.json stores exact helper/parent names, expected counts, completion-marker
contracts and validated working-source hashes only. Raw executions/commands/OS
metadata/source manifests are registered under private evidence:root/20261107061215.

## How to read it

The earlier failed whole-workspace probes stay failed historical evidence in the
runner owner's private evidence20261107051539. Narrow helper accounting accepted
in runner16c9e89 still rejects unknown ignored tests, missing parents, extra
summaries, failures or filtered tests. This local source correction is not a
successful rerun of the whole workspace or product e2e. Runner owns that rerun
against the canonical correction SHA after handoff. Working source hashes and
post-commit canonical blob equality are recorded separately; production APIs,
scoring and Cargo dependencies are unchanged.
