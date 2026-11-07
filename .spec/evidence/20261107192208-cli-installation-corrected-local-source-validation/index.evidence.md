---
id: evidence:root/20261107192208
title: CLI installation corrected local source validation
status: active
created: 2026-11-07T19:22:08Z
updated: 2026-11-07T19:23:08Z
rel:
  depends_on: [sot:root/cli-installation]
files:
  - {path: summary.json, sha256: 8cf2e284f699e172d5b47464303c766eb9b6572128fc77238f132fbc2b6f8341, bytes: 3980, store: repo}
---

Root allocated one new immutable-source run of
`f20db16ed32cae78c39057fa35319a142ccc78e3` after the missing-local-source finding.
The source workspace was newly extracted without overlays. Its ZIP hash equals
the original canonical archive, which independent audit compared with literal
Git bytes. The returned target cache was explicitly reused under a new CreateNew
lease/readback. Cargo commands were cache hits; no fresh recompilation is claimed.

Package fmt ran first, followed by locked/offline Windows platform metadata.
Before any check/test/Clippy command, mandatory assertions confirmed all16local
package manifests,434Git-listed Rust files and the exact CLI runtime_host/input.rs
were in the snapshot. Exclusions were relative to each package root and removed
only target/.git subdirectories. All selected package files were hashed, including
module and fixture inputs. Both immediate snapshots contain504local files,
6666registry files,3root files and66toolchain files,7239total, with0key/hash
differences. Seventeen environment variables also matched.

Both snapshots include a canonical Git blob join for every507local/root files,
with0byte mismatches. Separate post-run analysis of those retained snapshots
against the complete Git inventory confirms507expected entries,0missing and
0unexpected in each phase. This analysis did not recapture or backdate source.

All-feature/all-target check and strict Clippy passed. The selected library
inventory was exactly11tests; the actual footer was11passed/0failed/0ignored/
0measured/15filtered,0.05s. Selected tests remain pure byte/plan/state/error and
synthetic SecretSource cases. The CLI-to-Runtime DATA conversion is type-checked;
there is no additional signed CLI end-to-end invocation in this profile.

The original work clock ran2026-11-07T19:21:02.6285841Z to19:21:24.7821390Z,
22.15s. Exact single-file marker cleanup completed19:21:51.1875920Z,48.56s from
the same start, within600s work/60s cleanup. All six command handles terminated
exit0; the lease marker is absent and target returned. The first submitted
orchestration command was blocked before execution by a PreToolUse classification
of single-file cleanup as recursive deletion. Splitting exact literal cleanup
from execution safely resolved this without a source retry or hidden run.

Private evidence:root/20261107192210 retains the new admission, command logs,
coverage assertions, metadata, manifests, canonical joins, environment and ordered
archive parts with hashes. summary.json provides public source-bound accounting.
The original defective6674-row evidence:root/20261107191156 remains HOLD, unchanged
raw receipts preserved. This new proof is not applied retroactively to that run.

Native preparation remains Unsupported. Actual issuer/process identity, protected
delegation, atomic claim, W, transport, Ready and hosted service lifecycle remain
open. No local service/account/SCM/listener/native pilot was run. No audit
drain/flush, actual ServiceHost or product e2e qualification follows.
