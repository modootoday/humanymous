---
id: evidence:root/20261106224155
title: Container test environment research
status: active
created: 2026-11-06T22:41:55Z
updated: 2026-11-07T06:12:23Z
files:
  - {path: container-runtime-results.json, sha256: 82c63a579e44279bae8e6d832d483e57f375b307ec6df67b27adf9c9255da363, bytes: 3958, store: repo}
  - {path: provenance.md, sha256: 881be44b70ea201eaa97ab25f0d4ab5b0bacb8580fb5368d65d8dc7cfc8ce108, bytes: 884, store: repo}
  - {path: rust-runner-v2/lab-report.json, sha256: 75012d28bd66998abacf3d43efb84d58eefb31d69a226bee3b2ab09628eb801d, bytes: 4365, store: repo}
  - {path: rust-runner-v3/lab-report.json, sha256: 607ca8a93d15717ac91756405783c5a2bf192e1ed564592c5b07970e3409d6d9, bytes: 48532, store: repo}
  - {path: rust-runner-v3/witness-report.json, sha256: c9e9914dd4150aaceed3485d49d55cb4ae1efcedef6c8fb6f28a9d66eb60d64f, bytes: 3042, store: repo}
---

# Container test environment research

## What was measured

Rust-only continuation is separate from the historical v1 Python/Node record
below. Rust checkpoint4f89ec3 passed25 topology controls. Private source54017
passed33orchestration assertions, actual runtime origin2/audit4 and complete
wargame26scheduled/19evaluated/7unsupported,0errors/0mismatches. Fresh source60627
passed71assertions and the same wargame matrix, adding separate writer/witness
UID/volumes, actual external Log co-signing before effects, pinned recreation,
older signed-head rejection, wrong-UID EACCES and broker-unavailable origin0.
This is scoped component evidence; full product readiness remains false and
the required-full command exits2. Fresh image/secret/Cargo advisory closure is
pending after the final source corrections.

Failure attempts remain distinct:50621 concurrent-source build gap;55200 initial
setup source graph lost through an erroneous retry folder reuse, with its other
raws archived under55537 failure-*;55200's retry empty-attach failure;60036's
EOF-injection timeout. Do not map a later source manifest to the lost graph.
The54017 complete success graph/image/report has not been modified. Canonical
runner now refuses nonfresh run/audit evidence instead of overwriting it.

Official container/provider research plus a scoped Linux/amd64 Docker pilot on
2026-11-07 KST. The pilot passed 26/26 orchestration/topology assertions and
22/22 actual Rust authority/Express/Hono assertions. Its observed incomplete
application-HTTP verdict is CHALLENGE, not ALLOW; required raw TLS/collector/RIT
routes fail 422 at authority and 503 at adapters with zero origin effects.
Restart rotates credentials and authenticates WAL; framed WAL corruption stops
listener startup. This is neither a complete Gate/browser lab nor independent
witness or persistent-container recovery qualification.

Final fixture and authority images have zero vulnerability findings with pinned
Trivy 0.76.2, DB updated 2026-11-06T18:59:52Z. Cargo audit and the minimal
Express/Hono npm lock audit pass with hashes matched to the built source.
The earlier Python pip-vendor scan had 8 findings; the earlier Node24/Debian12
plus build tools image had 260. Removing unused installers/build tools and
using pinned Node26/Alpine with Rust musl produced the clean final scans.
Scanner authentication and undersized-cache failures are recorded as not_run,
along with the corrected stopped-container harness inspection failure.

## How to read it

Remote design refresh2026-11-07: container-test-environments SoT records current
official Containers pricing, limits, outbound policy and FAQ URLs, plus a
Compose/provider equivalence matrix. This is documentary research only;
remote execution, Rust-only provider lifecycle, sandbox/network/witness
equivalence and complete costs are not_run. GitHub-hosted Ubuntu execution
design belongs to the delivery owner. No local build or paid resource was
started for this refresh. The existing local receipts remain source-specific.

container-runtime-results.json is the reviewable summary with source/config,
image/export/config digests, scan/database time, denominators and unavailable
gates. Private evidence:root/20261106230622 contains registered raw Compose,
inspect, process, transcript, lock audit and image-scan results. Earlier private
bundles preserve intermediate failures and pre-remediation scans; they are not
aggregated into final success counts. The final command deliberately exits 2
and releaseReady remains false. Source manifests identify the immutable build
snapshot because the shared working tree was dirty.

No provider account operation, paid resource, public Tunnel, host port,
production change, Go maintenance or docs/** modification was performed.
