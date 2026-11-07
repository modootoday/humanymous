---
id: evidence:root/20261107052001
title: Exact-source full CI failure and Rust delivery follow-up
status: active
created: 2026-11-07T05:20:01Z
updated: 2026-11-07T07:34:29Z
files:
  - {path: summary.json, sha256: 753f1e86267897755bf84e5fd5d64157ca92e2c33b6dca661629686c9fa1557b, bytes: 5330, store: repo}
  - {path: hosted-63f69ae-summary.json, sha256: 48c6cfd54506776a04e9f6182d55c98351c6a3cbe49ceb4759d8da7ad979c9d6, bytes: 5707, store: repo}
---

# Exact-source full CI failure and Rust delivery follow-up

## Measured boundary

Full run42339951733 belongs only to
f32042af9136560b41c1765f565c2e04fe373a22 and completed failure. The attached
sanitized summary preserves all16job outcomes:4success and12failure, including
the required aggregate. Policy, Pages build, legacy Node checks and pinned
toolchain tests passed. All three native jobs reached actual compilation/unit
execution/release build and then refused the missing full native runner.
That is not a completed native product deployment or cross-OS qualification.

The same-source failures include missing browser/Docker runners, Go shipping
image rejection, publish=false packaging, stale documentation inventory, Unix
native-cli test-helper Clippy dead-code errors and frozen Go dependency findings
in the root filesystem scan. No later source's component success replaces this
run. Raw logs, Windows actor reports, exact image/build metadata and artifact
payloads remain private and are not attached publicly.

## Second source: actual free hosted validation

Run42348518353 attempt1 belongs only to
63f69ae9de703939b7b9091e2f215b12bea265de and completedfailure. Attached
hosted-63f69ae-summary.json retains all16terminal jobs:3success(policy/MSRV/
Pages),13failure,0skipped. Whole-run rawlogSHA256
5c03079b00af0704ec0387b82b27fc7e4178cd838a1737eb6f300863252d0e3c
is in private observation/attempt-1 evidence. Exact per-job logs are also private.
ArtifactsAPI returned0; cache inventory retained3older entries with no creation
or access since before this run. This is evidence for this run, not account-wide
zero charges or proof for unexecuted image scan paths.

Policy passed27puretests and exact public pin/catalog/environment checks.
MSRV and Pages passed; default Cargo tests do not establish strict native actor
qualification. All3native jobs failed in owner workspace/summary, with exact
harness unknown because public summaries were not emitted in this source.
Docker ownerexit2/errors0 was component-onlyblocked; browser ownerexit1/errors1
was a real failure with detailed private owner diagnostics unrecovered. Both
image jobs refused existingGo shipping source. Packages refusedpublishfalse,
Node qualificationreceipt wasabsent, quality failed native-cli Clippy helpers,
docs inventory wasstale. Trivy reported13Go-module findings(unknown1/medium7/
high5/critical0),other listed Cargo/npm/Rubylocks0. Raw Cargo audit reports and
native/lab private transcripts were not recovered; do not claim full auditclosure.
Ready correctly rejected failed requirements. Subsequent source diagnostics,
newproductbrowser/nginx/candidateimage jobs and adapter changes are not validated
by this run. No registry publish, tag, token exchange or image deployment occurred.

## Follow-up source implementation

The new Rust delivery tool separates source/working-byte commitments, workflow
authorization, package/registry barriers, owning-runner integration, dependency
closure and image provenance verification. It does not implement missing product
Core/Gate/Pass/human/AT functionality or external registry account setup.
Initial Rust boundary tests and own preliminary audit are component evidence;
final source-bound checks, shared Cargo integration and real owning-runner
pilots must be recorded independently after implementation settles.

No publish, tag, main merge, external registration or broad detection claim was
performed. The delivery plan remains active and full_product_qualified=false.
