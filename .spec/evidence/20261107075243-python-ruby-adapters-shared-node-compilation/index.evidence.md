---
id: evidence:root/20261107075243
title: Python Ruby adapters shared Node compilation
status: active
created: 2026-11-07T07:52:43Z
updated: 2026-11-07T10:15:19Z
rel:
  governs: [crates/node-adapter]
files:
  - {path: summary-001.json, sha256: 09ba1cdf3e041fbbf16f6ff1546e49a4261a55a3bae2e8c187fd6f21524f89d9, bytes: 1306, store: repo}
  - {path: summary-002.json, sha256: cf779ccd005511ab728655fcb2f185eec98368ece59cf722a4d59390ce1055cf, bytes: 1839, store: repo}
  - {path: summary-003.json, sha256: b4b9498c9d6693ffa948a1fd10a3fa5ca85d87f6a1b0e21dfa7d3298c0145958, bytes: 3905, store: repo}
  - {path: summary-004.json, sha256: fa91c3441f1618771c7b8f34404347728e053cdc7b79202901771230e6c8abfc, bytes: 1194, store: repo}
  - {path: summary-005.json, sha256: 781917464064e470b1b1b88e6c1c7a8e3fc5a7505d4e9fa0a01a19bb15c257e5, bytes: 2887, store: repo}
  - {path: summary-006.json, sha256: f4397ca167766758ffe5ff19aff3dba7a735c5b188fdf1ba990003261d6c51b8, bytes: 4789, store: repo}
  - {path: summary-007.json, sha256: 76fbd74edc155d3415e82d8f71f45335774d878c66a1c9db9ea6e13b73c273d8, bytes: 2629, store: repo}
  - {path: summary-008.json, sha256: b511ed53a3b304ec13377284924e276beb6a569345d5cc2fe299a60698f0bc57, bytes: 3114, store: repo}
  - {path: summary-009.json, sha256: ad1a83dfba2d2f953fdfe398fff5cc2b304a11ca949c3339d75b6d9e98e492b4, bytes: 2327, store: repo}
  - {path: summary-010.json, sha256: add5762fcb709a466af4976002d41bc46399f798d36539c7c44c3091606b39ec, bytes: 2763, store: repo}
  - {path: summary-011.json, sha256: 6d7e8054f717f621803acd8ec7628e4332692cdc9830813a9c4ba94008717477, bytes: 1963, store: repo}
  - {path: summary-012.json, sha256: b56d67b4cd5ca9c7a78d8d524bd033ef8bc739e9c9e97e92d15b640bc44734be, bytes: 1888, store: repo}
  - {path: summary-013.json, sha256: 1cf7b1c41eb5208d7d90ec90a7529163e6f7ffafed8feb042311862297b6eb3c, bytes: 2319, store: repo}
  - {path: summary-014.json, sha256: f7b69201c3c03dcf4d91a923efc29164ca6052cf948d5492ede27aa59f73d7e2, bytes: 3185, store: repo}
  - {path: summary-015.json, sha256: e11d4b23d5349cd02945c43c3b7b28b47df8bf748c99cdf7a40fdd6024eca0bb, bytes: 2057, store: repo}
---

# Python Ruby adapters shared Node compilation

## What was measured

Root explicitly assigned isolated local B target/node-native/build with
CARGO_BUILD_JOBS=1 and CARGO_INCREMENTAL=0 after its graph integration completed.
The Node all-targets host-pilot check passed in41.35s and scoped Clippy -Dwarnings
passed in3.60s. No actual Node/Bun/framework/authority host process was started.
Resource guard before check:34.79GiB available/82% committed; before Clippy:
34.26GiB/83%. B was returned immediately on success.

Owned source checkpoint78562be (including earlier Node migration44acbbf) was
clean at completion. HEAD observed after commands65166bb6e9b35caadac338978c593ceeb1a2a6ab.
This observed HEAD is not falsely presented as an immutable whole-tree execution
snapshot. Concurrent shared-dependency source mutation was not independently
monitored. Final hosted qualification still requires an immutable exact checkout.

Raw logs remain private; public sanitized receipt includes commands and hashes.
Cargo.lock SHA256 observed after completion:
06c2ed0db6d101a6e44c0e8186f550b2f565339ffe4a9ff849daddcf17146996.
Raw check SHA256 aed031ff295fba6331ec83bb31082e1aec9a64e20dcd897e46b1cfcf372e7caa;
raw Clippy SHA256324c091c91dbe0ae21097c52c0428902f4c93ac7ae2204d2d142f4af322b7b54.

## How to read it

Compilation/lint only. No unit execution, native loader/package verification,
actual runtime/framework/IPC/ABI/browser or whole-product qualification follows.
The retained older Windows52/Linux36 Node runtime receipts do not validate this
new shared-context source. All Python/Ruby native/package/runtime gates are
separately tracked; no default-empty-crate result counts as native success.

## Python native compilation follow-through

Root separately assigned the same isolated B target for Python all-targets
host-pilot. Check002 passed1.97s against source ba0c291. Clippy001 failed on two
collapsible-if diagnostics, then those two owned source expressions were changed
to equivalent let-chain/conjunctive forms and Clippy002 passed0.79s with-Dwarnings.
No lint was suppressed. This final Clippy invocation also type-checked the fixed
source. The initial E0277 check001 remains a retained failure, not overwritten.

Summary002 binds the final two changed source hashes and all three raw command
log hashes. HEAD observed after the commands5016be6a4a5702b77a7f586c2e948c645f75c883
is observation metadata, not a claimed immutable whole-tree execution snapshot.
No test/import/product/host process ran. In particular the required-feature
Python package-tool binary was excluded from this host-pilot build and remains
unverified. Ruby native and its new package driver are separate unfinished gates.

## Package drivers and shared Node fixture follow-through

Summary003 records six successful root-assigned B commands after the approved
graph window: Python package-tool all-targets check/Clippy, default Ruby Rust
driver all-targets check/Clippy, and Node host-pilot all-targets check/Clippy after
test fixture extraction. Selected184 source/common files changed0; excluded
transitive crates are named, so no full-graph stability claim. No tests, native
imports, frameworks or installers ran. Ruby default does not enable Magnus.
New Python Unix WSGI source was excluded by platform and feature cfg and has no
compile/runtime evidence from these Windows commands.

## Source813 hosted Node stage result

Summary004 retains final job128361689495 from run42355168729. Native build and
inspection stages exited0, then the required qualification.json was missing.
Exact813 package flow did not execute actual host_process tests before that gate;
native generation/loader inspection is distinct from framework/authority proof.
The LF-normalized API log remains private with its hash. Archive packing was not
reached; neither this historical result nor later source-only feature guards
qualify current packages or runtime tuples.

## Default Ruby pure tests and retained lint failure

Summary005 records a new assigned B batch: Ruby library2 and package-driver2
pure tests passed with zero failures/ignores/filters. All-targets Clippy failed
one items_after_test_module diagnostic. The batch stopped before Node commands
and B was returned. Conservative recursive closure tracking covered13packages/
251files and observed one changed test-support Docker source path; no full-graph
stability is claimed. Owned Rust stayed frozen. Moving the unchanged Drop impl
before tests is a later order-only fix whose retry remains NOT_RUN. Neither the
pure tests nor the default feature compile Ruby native callbacks, ABI or packages.

## Successful Ruby and Node retry with closure drift

Summary006 records the assigned retry: Ruby4pure and all-targets Clippy passed;
Node default and host-pilot all-targets check/Clippy passed, six command exits0.
The owned source was frozen. Conservative recursive13-package/252-file tracking
observed two audit paths change, with no additions/deletions. Therefore these
passes do not establish immutable whole-graph qualification. The earlier failed
Clippy attempt remains retained in summary005. No native Ruby, Node runtime/
installer/framework, or Unix Python WSGI execution occurred; Node pure parser
definitions were compiled by all-targets but not executed.

## First Unix WSGI type check on an immutable snapshot

Summary007 records WSL REA-Ubuntu native Rust check21.81s and Clippy2.22s against
source8eb66811dbb87a12a9d57e4e0c5048758efc1c4e. Intended source hashes matched
the primary and the whole snapshot's2202 regular-file rows were unchanged.
Coordinator-provided archive identity is explicitly attributed. Rust1.99.1 and
Python3.12.3/dev tooling were observed. No test executable was linked, no test or
native module/framework imported, no server/installer/Docker executed. This is
the first genuine Unix-branch type-check result, distinct from earlier Windows
cfg-excluded checks and all remaining actual28-case acceptance.

## Prepared WSGI artifacts; execution denied before launch

Summary008 records ordinary UID1000 private setup and three successful builds:
native fixture extension, production authority, and WSGI test executable. Exact
artifact/source/dependency hashes and setup versions are retained. The source
manifest here is Linux UTF-8/LF output, distinguished from the retained redirected
manifest representation in summary007. Prior setup transport failures are not
discarded. No global package changes or snapshot edits occurred.
Before the actual run, commit87.51% failed the <85% admission requirement. Thus
expected8Rust groups/28actual cases have no execution counts, and no native
framework/server import succeeded or failed. A was returned and prepared private
artifacts preserved for later explicit allocation; this is not actual acceptance.

## First actual WSGI execution: two fixture failures retained

Summary009 records the later allocated ordinary-user execution of the same
immutable source and prebuilt artifacts. Eight tests ran: six passed, two failed,
none ignored or filtered, exit101. Four actual groups and two pure tests passed;
ten completed scenario rows cover22 actual cases, short of the required28cases
and14rows. Flask and Django both exercised real native imports, Waitress HTTP,
authority IPC and audit on successful groups. This is component evidence using
test-only frozen evidence, not production browser or whole-product qualification.

The response-timeout fixture failed to prove actual request acceptance; its
socket lacked the client's required0600 mode. The incomplete-body fixture
attempted to reopen empty audit history, which production correctly refuses.
Primary fixture fixes preserve accepted-request proof and exact owned empty WAL
bytes and identities without changing production semantics. Corrected source
has not been executed. Source and dependency manifests were unchanged; artifacts
passed exact hash checks, task-specific cleanup found no remaining owned process,
and allocation B was returned. The first failed run remains authoritative history.

## Corrected immutable WSGI retry: all required cases passed

Summary010 records source7842b6a8 extracted from a separately hashed git archive.
The new module, production authority and test executable were coherently rebuilt
with locked offline dependencies and passed exact hash checks. All-targets check
and Clippy passed. Under ordinary UID1000, all8tests passed with no ignored or
filtered tests:6actual groups plus2pure tests,14scenario rows and28actual cases.
Source2254-file and reused private dependency manifests stayed unchanged before
and after execution. The owned-process scan was empty and allocation B returned.
The earlier failed attempt remains retained. WSL component success establishes
neither hosted execution, production-browser evidence nor whole-product readiness.

## Immutable refactored-source compilation found a missing import

Summary011 records the renewed compile/parser-only allocation on exactc5de4ae5.
Admission80%commit/free32.42GiB passed. Ordinary UID1000 all-targets check failed
E0433: the shared case extraction omitted its parent http module import at two
uses. The batch stopped with101; Clippy and the two receipt pure tests did not
run. No controller, native framework import, server, package install or actual
pilot ran. All2302source-file hashes stayed unchanged. B returned terminal101.
The primary-only missing-import correction requires its own new checkpoint and
retry; no passing result is inferred from formatting. Historical WSGI28 success
remains tied to7842b6a8; shared lifecycle refusal remains NOT_ACCEPTED.

## Refactor type check passed; strict receipt Clippy failed

Summary012 records the renewed ordinary-user immutable5655b965 batch. All-targets
host-pilot check passed3.01s, covering the new ASGI factory and shared WSGI/ASGI/
controller definitions. Clippy then failed one sliced_string_as_bytes diagnostic
in the receipt parser; the batch stopped101 and neither receipt pure test ran.
Source2313-file closure stayed unchanged. No linking, framework/native import,
server, controller, package install or actual pilot occurred. B returned terminal101.
The primary-only equivalent byte-slice correction has no suppression and needs
its own new immutable retry. This is type-check evidence, not runtime qualification.

## Strict compile and two receipt pure tests passed on immutable source

Summary013 records exactd68682e4: all-targets check2.33s and Clippy2.68s passed.
Receipt test linking completed6.25s, then the explicit receipt::tests:: selector
executed2passed/0failed/0ignored/2filtered in0.00s. The filtered definitions are
the blocked controller/cleanup entry points, not omitted actual server cases.
The tests accept allowlisted historical14-row data and reject incomplete,
duplicate, ambiguous or mutated receipts/discovery. This is pure parser proof.
Source2325-file closure stayed unchanged; batch/closure exits0. Handle62759
exited and B returned terminal0. No setup/import/server/controller/pilot/install/
signals occurred. All previous failures remain retained. Typechecking the new
ASGI source and running these two pure tests does not qualify actual ASGI42,
refactored WSGI28, packages, hosted execution or the blocked lifecycle API.

## CA-API-01 admission construction invariant verified and independently closed

Summary014 binds the private-field fix to exactac7dc4a9 and fixed file SHA256
e0a012386d84fe2d4c61be5e5f96e35247d2f3ce189a194ab64db6ae923858e3.
Scoped check15.03s/strict Clippy1.98s passed, as did6units and2external rustdoc
compile controls (supported acquire/no-run and original direct-constructor
compile-fail). All footers have0failed/ignored/filtered. Source2522/registry20266/
configuration11 before/after ledgers matched exactly; batch exited0 and C returned.
Readonly broad fmt was disclosed; it did not compile/test the whole workspace.

The independent auditor subsequently reread the exact canonical source, raw
output, public summary and actual native before/after ledgers via read-only UNC,
confirmed the framework leaf hash and literal batch.exit0, and accepted closure
of the safe external Rust API ownership invariant. Summary014 preserves its
earlier generated pending-review wording; this later disposition supersedes that
wording without rewriting the audited attachment. No remote HTTP, SDK/host,
controller, native lifecycle or full-product qualification follows.
