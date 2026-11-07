---
id: evidence:root/20261107052815
title: Pages Rust verification and standards experiment status
status: active
created: 2026-11-07T05:28:15Z
updated: 2026-11-07T10:00:42Z
rel:
  governs: [docs]
files:
  - {path: status.json, sha256: 08db59f7c4b992ab7fa8d7a2ee859002c5b31270629f85a38a5cf0864c397d34, bytes: 866, store: repo}
  - {path: freshness-status.json, sha256: 82ebeec598747454d06f4c67ddf286161a688dd65d13da045e66f7f284940e24, bytes: 1600, store: repo}
  - {path: i18n-status.json, sha256: f627ae8f812217a396d97f1cba9ef83952b2e3b92b8788142d5cc3dde46a5ee3, bytes: 831, store: repo}
  - {path: cli-review-status.json, sha256: ad116a0ae6e7725205d4146bc40681db6e3531d53738347c2b4eb10684265d70, bytes: 1313, store: repo}
  - {path: audit-reader-status.json, sha256: 30a69bec28f952ef3e75e6d23853f867499aaa29fc2a9e129d81941bb435ba0f, bytes: 824, store: repo}
  - {path: native-planner-review-status.json, sha256: 8434f3e1b58e85824ccdfe38bee13afa3f7889744a3f57b472e2d29faceb1415, bytes: 807, store: repo}
  - {path: resource-hold-status.json, sha256: 491dcae56b56bf43e2302489e53a7014025618a147c87b013909f026f2d809d3, bytes: 1407, store: repo}
  - {path: navigation-source-status.json, sha256: 015dca70cd44a0abb0b37058f33d0fef0844ef3f4baee04c55161de6f7259b0c, bytes: 889, store: repo}
  - {path: reader-source-status.json, sha256: 885098b2dfcbb0853ca93028d60084f274edd9f8e1796eae645ffce96f4b5b3a, bytes: 1151, store: repo}
  - {path: output-checker-attempt.json, sha256: eb234eee5d028f42a4a7e3ad3a19bdca00d31c2c5c27e87f36bbcd18f13d8c8e, bytes: 3009, store: repo}
  - {path: hosted-63f69ae-status.json, sha256: 66c81ccabb8d11f70fc2227f93b97f5b57246bbe2b8d7ff174bc2ee4887ba343, bytes: 1523, store: repo}
  - {path: english-authoring-scope-status.json, sha256: 19b7b53753a9a4ba87feb77ca582e3b114700afe75af0615bb9f62297544c72f, bytes: 894, store: repo}
  - {path: browser-integration-source.json, sha256: c772e25f8e51669e598bd3b4cfac1e48144439dd55504198a6f056d2476b93ed, bytes: 2609, store: repo}
  - {path: official-reference-editions.json, sha256: d8d9d7105ea875ee938e948ff3810b3f87a7b2f42608ad926766ab6e745aab76, bytes: 1717, store: repo}
  - {path: hosted-a26d1c3-startup-status.json, sha256: 20c60494b1f018e8b76cb67383f78b9fe146b81cc6d1c58b0871e2dce9e6b1aa, bytes: 1156, store: repo}
  - {path: hosted-8132117-initial-status.json, sha256: 6f8efcee1480f35d5d5090c0afbc5242b4cb4789f0b991d42a398f8077dce50e, bytes: 1448, store: repo}
  - {path: hosted-8132117-partial-failures.json, sha256: 18b96b0529229f3431313d271b5303c89c3870a69f509c54115c0216a7a6eeaa, bytes: 1106, store: repo}
  - {path: browser-dot-segment-verification.json, sha256: c0d9e1fae7db02a3f9921b89acf1f7933cb97bde61d9891fdda6837f2725b461, bytes: 642, store: repo}
  - {path: hosted-8132117-nginx-windows-status.json, sha256: a9ad0f8d4963ea158586f3ccbd18ac33ec4cd10b2c77d248852b2d962c0ec75d, bytes: 1785, store: repo}
  - {path: public-terminology-current-source.json, sha256: 31b63719302fb1cd213cb510e44967390e9e39265632df9ae4f07d5d65e519f4, bytes: 944, store: repo}
  - {path: normalized-profile-fixture-static-status.json, sha256: daf9aa6caec6d06a03b1fb595f07b877d7edb0f288703eb6309d4d98f1bea101, bytes: 636, store: repo}
  - {path: hosted-8132117-pages-build-status.json, sha256: d989e7932ec76d4a46457893acbc66d052a0b40f934e487a3ea650663f161c2e, bytes: 1000, store: repo}
  - {path: hosted-8132117-authority-subset-status.json, sha256: 455a044de376b377c846357ce40f01b429fab83ac4697b057fd4fe414c6108f9, bytes: 775, store: repo}
  - {path: hosted-8132117-final-status.json, sha256: b8bbf69e1ecfe4bb287f5fdcc4f5b20659d2f4c8623b62a5734657b63e4fcfa4, bytes: 3074, store: repo}
  - {path: normalized-profile-execution-status.json, sha256: dba332da14b8f246bb3e87f2160a9d767e7bcf67312b524ec1bc2e6bbd12d6b6, bytes: 1864, store: repo}
  - {path: windows-ruby-prerequisite-attempt.json, sha256: eecdfcd0cadf60ef714409096dc771e768a8a490a88aba193bb53346a797d942, bytes: 1111, store: repo}
---

# Pages Rust verification and standards experiment status

## What was measured

The authored Pages tree contains 85 reader pages. Actual pinned-theme Jekyll
builds and upstream strict checks succeeded for both the empty baseurl and
`/humanymous`; the Rust DOM/link/anchor validator reported no errors. A later
apex rebuild includes the explicit collector guard-hook compatibility text.
The current in-app preview was visually observed at localhost port 8766.

The Rust Pages package passed 11 unit tests with no failures or ignored tests,
and strict all-target Clippy. The fixed experimental adapter parser corpus
passed 4 accepted and 20 rejected cases. Its SHA-256 is
`b1fc0069ef858ce36cf9cb6810a5852cfed73559a44fc6f2797f50232b11ed92`.
These are parser outcomes, not enforcement verdicts or Draft v0 conformance.

After those passes, a small change moves invalid-baseurl rejection before any
builder process. Its initial rerun was blocked by concurrent common workspace
manifest/lock drift under `--locked`; the failure is retained. After the common
lock was synchronized, all 11 tests, strict Clippy, source checks and 24 vectors
passed again. No dependency or lock changes were made by the Pages owner.

## How to read it

P0 browser/runtime source follow-through is now documented separately from the
pure collector Session. Working declarations add BrowserSession/BrowserResponse
and explicit runtime browser policy/bootstrap/ACK handling, but no changed-source
build/pilot was run under the hold. Latest working source preserves Authorization
for BrowserAuthOnly and inserts status-bound ACK headers into local and parsed
origin responses. This supersedes an earlier in-progress observation; credential
preservation and counter-ACK integration remain execution-unqualified. New source
types/policy intent are not a working flow. The lightweight existing source
checker reports86pages/errors0; build/render/AT remain not_run.

Native-planner source/prose review covers all11package files and manually
authored per-file receipt reasoning with observed LF source hashes. It corrects
the tutorial's workspace-ownership-check overclaim: the writer pins directories
and rejects path/reparse aliases, but does not inspect workspace owner/ACL policy.
It also distinguishes published destination from temporary cleanup, exit3 and
staging residue after failures. No native tests/build were run under the resource
hold, and this partial review does not accept the entire source inventory.
Existing compiled source checking reports86pages/errors0; actual new-tree build
and rendered accessibility remain not_run. Native-planner review status and raw
selected hashes retain that scope, without an SDK/product qualification claim.

During the coordinator's memory-related heavy-execution hold, current native
authority client/handler/drain interfaces were reconciled in public prose and
the proposed retry/effect lifecycle was authored. The existing compiled source
checker reports86pages/errors0, with executable identity in resource-hold-status.
No new compilation, Jekyll build, browser execution or global spec check was used.
The new tree's actual build and rendered accessibility remain not_run. Earlier
85-page build/input commitments do not cover this new86-page draft.

Further light source inspection identified tolerant TLS-prefix parsing and the
first-offered ALPN compatibility field as distinct from complete capture and
negotiated protocol evidence. The topology page now records those distinctions,
HTTP2 passive observer limits and capture-owner reliability duties. Existing
compiled source checking again reports86pages/errors0 after the edit; no network
test, compiler, actual build or browser was launched. The rendered gate remains
open and no network source receipt was renewed from partial source inspection.

Audit public-interface reconciliation now distinguishes omitted HMAC, zero
checkpoint/witness coverage, native scope omitted from compatibility JSON, and
terminal-frame repair from read-only verification. The documented four frozen
parity targets ran5tests with no failures/skips/filters. This does not renew the
whole audit package receipt or prove process isolation, pre-ingestion completeness,
remote export or product qualification. Selected source hashes were observed
after these runs; no atomic full-workspace capture is inferred from that list.
The85-page dual-baseurl strict/theme/output builds pass with equal inputSHA256
`104a800c2d52aeae12c5d3a6a98aa4d151bf99feaa2a64523dbd39f0462600df`.

A subsequent substantive review manually renews18CLI source entries and two
capability mappings. All18files were read against the command/installation/status
pages; receipt entries carry file-specific reasoning. Windows15library+1command
tests and one selected exact read-only/actual-loopback-process case passed. Two
other process cases were filtered; no native service was registered. Provenance
metadata was read separately from approval. LF receipt hash and full remaining
inventory counts are in cli-review-status.json:154byte-identical entries,
244added and65changed. None of the18CLI paths remains added/changed in that
capture, but this is not acceptance of all other governed sources.

The actual release checker remains blocked. It currently enumerates generated
nginx build-cache files as source before reporting remaining substantive gaps;
CI owner was notified, and caches were not added to authored review records.
The updated reader review/versioning page clarifies that named checks are not
proof of execution and rendered-site exclusion is not repository confidentiality.
Its85-page dual-baseurl build checks pass with input commitment
`2c2122b9935e47362ed9b840017b8a9697605330cf95a4be8888fe5cd19e2c27`.

The supplemental internationalization corpus passes6accepted/4rejected with no
mismatch, separately from the unchanged original24cases. Fifteen Rust tests
and strictClippy passed. The final source also documents the actual collector's
explicit console probe and pure one-flight Session/ACK state, without claiming
Fetch/bodyreader/abort/server-wire integration. Actual85-page apex/subpath
builds passed strict theme and Rust output checks with equal input SHA256
`d7bd2c0cc19db9ce5ef662c7c615b10e3e2aae9923f3216ac2ed2e4f0d58779a`.
The vectors page's supplemental section and exact byte table were observed in
the in-app browser. This rendering observation does not add a full AT result.

Subsequent local-input freshness builds passed for the same 85 pages and both
baseurls, reporting the same exact input commitment
`8df2f91fe3c572c97a2a55560feb0d7d0f5f67a3d75c599eff80cc1175ad4881`.
Navigation/configuration/Sass/binary assets are included. Installed dependencies
and verifier metadata are outside that commitment; external provenance remains
separate. Fourteen Rust tests and strict Clippy passed before final report-only
receipt-hash metadata was added. That metadata rerun encountered renewed shared
lock drift; after synchronization, 14 tests and strict Clippy passed again in
separate confirmed logs. The failed locked runs remain retained.

Read-only review triage observed247added/65changed/136byte-identical/0removed
Git-visible governed sources against the old receipt. It did not accept review
or renew a receipt. The updated command page was observed in the in-app browser:
Tab exposed a visibly focused skip link, Enter moved focus to main content,
next Tab reached a breadcrumb, and Enter expanded/collapsed Reference navigation.
This is one desktop manual observation, not full Pages, screen-reader or product
recovery qualification. The structured freshness-status attachment retains scope.

The later successful triage binds the exact baseline receipt digest and observed
dirty source revision, and finds256added sources rather than the earlier247.
Both results remain historical observations of a changing repository. The final
input-exclusion correction includes nested public assets with metadata-like
names; all14tests/strictClippy and both actual builds passed afterward with the
same local input commitment. No receipt was renewed by that result.

Private raw build/test logs are retained in the current Pages verification
bundle. Earlier failed invocations remain retained rather than overwritten as
successes. The source-review receipt remains stale and publication remains
blocked. Build success is not an authored review of every changing Rust source.

The public `P-01` through `P-10` identifiers are explicitly defined as this
project draft's profile requirements, with descriptive headings and validation
links. They are not priority levels or externally assigned standard numbers.
Generic wire negotiation, authentication and replay semantics remain open;
the implemented adapter experiment does not resolve them by implication.

No new complete accessibility, ordinary-human, screen-reader, deployment or
cross-implementation conformance result is claimed. The current browser check
observed the homepage and protocol-profile rendering only.

Navigation source guard follow-through: navigation-source-status.json records
a resource-reviewed, one-Cargo pure-unit check under the finalized local/hosted
policy. First attempt15pass1fixturefailure retained; after fix16pass0fail and
lib-only strict Clippy pass. Rebuilt source checker covers88pages and constrained
menu structure/targets. This supersedes no historical result and is not actual
Jekyll/rendered-link/browser/AT qualification; those remain not_run for88pages.

reader-source-status.json records the later reader-control guard:17pure lib tests,
strict lib Clippy and88page source/nav checks pass under one-Cargo resource policy.
Unicode/CRLF acceptance and control-byte refusals are scoped source checks; full
review freshness and Jekyll/rendered-link/browser/AT results remain unqualified.
