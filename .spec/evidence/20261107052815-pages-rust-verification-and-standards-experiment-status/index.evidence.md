---
id: evidence:root/20261107052815
title: Pages Rust verification and standards experiment status
status: active
created: 2026-11-07T05:28:15Z
updated: 2026-11-07T05:57:51Z
rel:
  governs: [docs]
files:
  - {path: status.json, sha256: 08db59f7c4b992ab7fa8d7a2ee859002c5b31270629f85a38a5cf0864c397d34, bytes: 866, store: repo}
  - {path: freshness-status.json, sha256: 82ebeec598747454d06f4c67ddf286161a688dd65d13da045e66f7f284940e24, bytes: 1600, store: repo}
  - {path: i18n-status.json, sha256: f627ae8f812217a396d97f1cba9ef83952b2e3b92b8788142d5cc3dde46a5ee3, bytes: 831, store: repo}
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
