---
id: evidence:root/20261107075403
title: Capture local pure units and default-feature Clippy
status: active
created: 2026-11-07T07:54:03Z
updated: 2026-11-07T07:54:03Z
files:
  - {path: final-validation.log, sha256: d1e301773bf9b48a5e8629fc8058a57f756cfaa28c9516690cbdf0bb403b6283, bytes: 2528, store: repo}
  - {path: receipt.json, sha256: 9b56331369625f627a8b61f600cb09870eca737dc2c94f5d6640604c892243c2, bytes: 2766, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107072531]
---

# Local component validation

Root ran the default-feature capture all-targets Clippy with warnings denied,
then locked offline lib units, in pipelineA with buildjobs1/incremental0.
Completed command exit0 is directly observed in root's command output. Lib
result:21passed,0failed,0ignored,0measured,0filtered. No socket was opened by
these pure unit tests. Owned direct rustfmt and diff whitespace checks passed.

Root's first Clippy attempt found4diagnostics (two nested TLS branches, nested
CONTINUATION validation, constant SETTINGS chunks). After those fixes the next
attempt found1fixture initialization diagnostic. All5were fixed without lint
suppression or policy changes. Original root command history and its earlier
capture-export-pages-clippy.log remain unchanged; the latter is the failed
fixture run, not the final successful Clippy output. The public final log is
sanitized from the later completed root command; exact original tool receipt is
retained privately under the matching network-capture evidence directory.

The receipt pins owned Rust source hashes observed after final validation and
records root's confirmation of the final struct-literal fixture. It is not a
full-workspace pre/post stability receipt. The subsequent owned source commit
must contain those same Rust bytes; source changes require another scoped gate.

| Remaining gate | Actual status |
|---|---|
| hosted-pilot feature compilation | NOT_RUN |
| Actual hosted socket/TLS/H2/cancellation cases |5authored,0executed |
| Runtime same-request/stream binding | Not implemented |
| Docker topology, required planes, denied-origin effects |0runs |

This proves pure parser/component validation only. Earlier source-only receipts
are historical and retain their actual0execution counts. No T4/human identity,
original-hop deployment, collector, runtime or full product success is claimed.
