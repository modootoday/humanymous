---
id: evidence:root/20261107052001
title: Exact-source full CI failure and Rust delivery follow-up
status: active
created: 2026-11-07T05:20:01Z
updated: 2026-11-07T11:55:34Z
files:
  - {path: summary.json, sha256: 753f1e86267897755bf84e5fd5d64157ca92e2c33b6dca661629686c9fa1557b, bytes: 5330, store: repo}
  - {path: hosted-63f69ae-summary.json, sha256: 48c6cfd54506776a04e9f6182d55c98351c6a3cbe49ceb4759d8da7ad979c9d6, bytes: 5707, store: repo}
  - {path: hosted-8132117-summary.json, sha256: 92b51d1914a16305b845afa92d2cf4073076b58eb72d961bf8aa4b695d42b17f, bytes: 7453, store: repo}
  - {path: apache-workflow-review.json, sha256: 822d27f101456cd6888327410ae3fe97c38d4376e7c140f5c25ba07d73781c9a, bytes: 945, store: repo}
  - {path: pages-8624b57-summary.json, sha256: 68c72b8bf92dd11e7acd0f10c68d2779f5abaee2b08ce3e8f1427777e72c4079, bytes: 2857, store: repo}
  - {path: go-438a3fee-summary.json, sha256: d0c9cff601c48cfd11c6c978138006b352cd446d155ebf02451453e2caefdd6e, bytes: 5911, store: repo}
  - {path: pages-44876534-archive-summary.json, sha256: 75bf113b3a71d9eecf3e2a6d668909e906e2f0d17a9caa5f3de897f794e5fd8c, bytes: 4478, store: repo}
  - {path: go-44876534-summary.json, sha256: 1a28cc8e1568fbf823d99fbd772d65990a74411f4cd1ca0dcf4cab2713413a99, bytes: 6201, store: repo}
  - {path: go-15f63fef-summary.json, sha256: 5b20b0714e544cd1393c36391d1a1615298eee2ffe229a81e80069b436178ee3, bytes: 6172, store: repo}
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

## Third source: rejected before job execution

Run 42354873001 attempt 1 belongs only to
a26d1c3f08b365ca93545c6a51343095868edb45 and is terminal startup_failure.
The Jobs API returned zero jobs and Artifacts API zero artifacts. No test,
wargame receipt, image build, native actor or cache result exists for this run.
The public run annotation identifies ci.yml caller actions:none versus nested
policy actions:read. The corrected caller source requires a new run; this
failure remains immutable and is not a failed product test.

Private API response captures are retained under
.spec.local/root/evidence/20261107082000-delivery-startup. Their SHA256 values
are run.json b1300c8f5291f7af32b67df4bd8005287ed2c6f69560adf255e7f1875609ca9b,
jobs.json 016130407ca1c61f7ee071fab174cc86c2e07c97627c504be864e65beebcea41,
artifacts.json e338fb10e6740e1e43cdbe137da3a22ec09f95d8097c5f247e751de5f81f87b4.
These hash the saved API output files, not nonexistent execution logs.

## Follow-up implementation boundary

## Later exact-source partial observation: 8132117

Run 42355168729 attempt 1 is still being observed; this is not a terminal
whole-run summary. Completed Nginx job 128358355202 succeeded in seven minutes
three seconds. Its safe owning summary reports 35 passed, zero failed/not_run,
cleanup exit zero and full_product_qualified=false. The measured image ID is
sha256:49cd270fab32122033cd630add92f8c5bb67e328f34e65558a6df808ec1657ab;
module SHA256 a911735564a23f646e2d7242d6a68f3b37a5622f860221b9ec19e92ab8f74845;
source manifest 20b56673e627209beb263f100071793effb8122cfe5467ae9bb9b6a6ee00770f.
These are bounded Nginx component results, not whole-product qualification.

Runtime image-candidate job 128359821189 also succeeded, proving its source build
but not vulnerability coverage. Build output records config digest
sha256:717d482cf97a4b02c120d8de7fec86896264b0bf2ed129890d65eb4ad826e0f9
and manifest digest
sha256:571a7b3cc2b50cd05e96d483c1d63b8ae6ca3724724981660ed405e6b1f14e34.
Trivy reports zero language-specific files and an all-dash target table, explicitly
meaning not scanned. Do not relabel that result zero CVEs or verified shipping.

Windows native job 128351926153 failed after the workspace build. It measured
15 of 89 harnesses, 55 passed tests, zero failed tests, four raw ignored tests,
one execution/accounting error and incomplete coverage. The owner independently
recovered its sanitized actor cleanup results; SCM was not executed. The three
ignored Node runtime groups remain required product coverage, not helpers.
Quality failed audit's items-after-test-module lint at control/retention.rs;
documentation rejected stale governed review inventory; both legacy image jobs
rejected existing Go shipping source; package readiness remained publish=false.

During the local benchmark quiet window, completed-job logs were recovered using
the connected GitHub service without launching local CLI/build/runtime processes.
Private *.connector.json captures retain decoded log text with an explicit
non-HTTP-wire-byte designation. Their local file hashes and final whole-run
evidence registration remain pending; no absent hash or raw owner body is invented.

## Source-only implementation boundary

## Terminal observation: 8132117

The later complete REST response confirms run 42355168729 attempt 1 completed
failure at 2026-11-07T09:24:11Z. Jobs total_count=19 equals the complete returned
array; every job's run_id and head_sha matches this exact run/source. Five jobs
succeeded, fourteen failed and none skipped. Artifact total_count=0 also matches
the empty array. hosted-8132117-summary.json records this final accounting without
replacing the earlier partial observations or prior source failures.

All nineteen individual completed-job logs are retained as captured CLI or
connector-decoded text, plus terminal run/jobs/artifacts REST JSON. This is not
an acquired whole-run archive or HTTP wire-byte claim. Local capture hashes and
registration remain pending during the benchmark quiet window; the final public
summary explicitly leaves those fields unverified/null. Original private owner
reports and bodies remain unrecovered.

After the quiet window ended, the existing delivery binary registered every
captured file in the private bundle without compilation or workload execution.
The capture index SHA256 is
8606d896814ab69dcb9f18766d6b11df1fdb09dc6a930a810d6e61392e49c540.
These hashes cover saved CLI text and decoded connector JSON wrappers, not
HTTP wire bytes or an unavailable whole-run archive. The public final summary
now records registration; missing original owner bodies remain unverified.

Cargo audit actually reported four source locks: root 670 dependencies/zero
vulnerabilities/one RUSTSEC-2026-0192 warning, CEF pilot 22/zero/zero, docs review
22/zero/zero and Windows runner 43/zero/zero. These overlapping source inventories
are not summed into unique dependency counts or promoted to an image SBOM.
The later filesystem scan retained thirteen frozen-Go findings. Ready correctly
failed the aggregate. No package upload, token exchange, tag or deployment occurred.

## Later source boundary

The new Rust delivery tool separates source/working-byte commitments, workflow
authorization, package/registry barriers, owning-runner integration, dependency
closure and image provenance verification. It does not implement missing product
Core/Gate/Pass/human/AT functionality or external registry account setup.
Initial Rust boundary tests and own preliminary audit are component evidence;
final source-bound checks, shared Cargo integration and real owning-runner
pilots must be recorded independently after implementation settles.

No publish, tag, main merge, external registration or broad detection claim was
performed. The delivery plan remains active and full_product_qualified=false.

## Owner-contract consumers

Root-allocated C passed final 55 delivery pure tests with zero failed, ignored
or filtered and all-target Clippy with warnings denied. Admission was 79 percent
commit and 35,296,160 KiB free physical memory, jobs one and incremental off in
exclusive target validation-ci-delivery. Final source/contract hashes matched
before and after; the first passing attempt's unused-import warning was retained
before the final clean retry. Private evidence is
20261107095612-delivery-owner-contracts, whose registered index SHA256 is
e99644994438ae077f1f01fa5e20c6404b3ac5a676b81b009020e6dde5d86e92.
C was returned and no actual component pilot ran. Actionlint and owned scoped
Specfold checks passed. The full pre-commit Specfold check encountered a foreign
Pages attachment registration error and binary-secret-scan warning; it is not
reported as globally passing.

## Later bounded Go and free-branch policy

Root-allocated C passed 64 delivery pure tests and all-target Clippy with warnings
denied after the lab's 99dc5606 bounded-mode contract landed. All test counts had
zero failed, ignored or filtered; source/workflow/owner-contract hashes remained
unchanged. Admission was 79 percent commit and 35,587,176 KiB free physical
memory. C was returned. Private evidence
20261107103309-delivery-go-bounded-verification has registered index SHA256
d381358cf50bf6add05aac3b98130a2117b50ffd12623dbc2d71d9c8645bae5e.
This proves parser/policy units and compilation, not actual Go8 or Docker/SCM.
The attached Pages terminal summary distinguishes real build success from the
historical automatic-upload policy miss and failed downloaded-site roundtrip.

## Pages archive and CEF consumer validation

Sequential root-allocated D passed delivery65 pure tests/strict Clippy and CEF
runner6 tests/all-target check/strict Clippy, with zero failed/ignored/filtered.
The earlier CEF estimate of four missed two existing provisioning metadata and
mock-file tests. The 61-file input closure was unchanged, admission measured
81 percent commit/33,666,764 KiB free physical memory, and D was returned.
Private 20261107105913-delivery-pages-cef-validation index SHA256 is
b29903d49f5bdeda8583e174c2cd9a1b74d385c26cd5545db6b35638f7dde502.
No SDK/native GUI/process/Docker or fresh complete-site archive ran locally.
The attached Go first-run terminal summary preserves eight NOT_RUN and an
unknown private compiler cause; successful prior units do not qualify Go8.

## Fresh public-site archive execution

The attached pages-44876534-archive-summary.json records actual successful run
42367274252, two successful jobs and zero artifacts. Preflight delivery65 and
Pages37 passed; both base builds and both package/second-consumer checks passed
with unchanged clean source and 165 public files per archive. Owning review
counts are 88 rendered pages per base with zero errors. Private decoded capture
index SHA256 is f224faca4b6fc7a3b336d750c1c99066eb23a19eea05a25ec05a8bc01d99b1cd.
Exact tar/manifest/inventory hashes are retained in the safe summary. Actual
tar bodies were not acquired, branch uploads stayed off, and main serving
transfer/browser/AT/full documentation review freshness remain unqualified.

## Distinct Go diagnostic results

The attached go-44876534-summary.json preserves the second runtime_build failure:
eight NOT_RUN, zero evaluated and successful cleanup, with no acquired private
compiler transcript or proven cause. The later go-15f63fef-summary.json records
run 42369171124, successful Go10/lab16/delivery65 preflight, and eight attempted
actor-start failures at actor_writer_mount. No case was evaluated and assertion
counts remain unknown/null. Cleanup succeeded, source stayed unchanged and the
typed requirement remained false. The marker identifies stage entry rather than
an exact failing operation. Normalized REST data and decoded connector log
wrappers are retained privately under evidence:root/20261107115500 with index
SHA256 97a47bf664f0634dcffea2b41d481a5b3e9316e175768a81bef0e4b282c94aa7.
There were zero artifacts; original private owner report bodies were not fetched.
