---
id: evidence:root/20261108043216
title: Preimage-bound ingress TLS H2 observation source candidate
status: active
created: 2026-11-08T04:32:16Z
updated: 2026-11-08T04:39:07Z
files:
  - {path: manifest.json, sha256: f31dd7ee85e10664e2d4ca62e7187d5e60b896cf0fa2875007b1da7f3a083605, bytes: 3104, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107142752, sot:root/structural-development]
---

# Source candidate, not applied or executed

Root assigned ingress completion while preserving the integrated eight-package
tree as read-only. Work stayed on codex/rust. Six candidate files exist ONLY in
.spec.local/root/evidence/20261108043216-network-capture-ingress-candidate/candidate.
Three replacement preimages still match the exact read-only tools-integration-
5f1010d4 tree; three new targets are absent there. Manifest lists every target,
preimage/postimage hash, size and LF status. Root alone applies this candidate.
No source/manifest/Gateway/branch/ref/worktree mutation or workload allocation.

## Integration contract

Network ingress projects bounded decoded fields into HeaderInfo DATA, leaving
order/casing reliability FALSE. H2Claim contains parsed application claims only.
Required method/authority/scheme/path must exist exactly once; request mismatch,
contradictory Host, invalid/overflowing Content-Length, repeated projected
singleton, unsupported values and field/byte/cookie-name budget overflow refuse.
Unknown regular duplicate fields preserve names/order. Repeated Cookie fields
and even empty trailer blocks are deliberately unsupported in this first profile;
they are not bot verdicts or changes to the frozen detector.

Capture Connection::observe_h2 is the only constructor of H2HeadersObservation,
which privately borrows the same live Connection. It requires ACTUAL negotiated
h2, selected valid stream, complete initial fields, Ended input, no reset/trailer
section and exact body-byte count. Offered ALPN, pure decoded DATA or caller
booleans do not mint this view. Reliability is set only from the actual owner's
fields. The borrowed view has no Clone/Deserialize/DATA conversion constructor.
Snapshot checks revocation/deadline before and after copying observations.

IngressSnapshot is historical TLS/H2 DATA, not a native proof or Ready lease.
It does not synthesize operator/TCP/collector planes into default healthy booleans.
No aggregate NetworkReport is silently built; Runtime must compose required
planes with the exact retained owner and recheck before effects. Body length is
not a digest/application-body association proof. Complete H2 serving/flow control,
original-hop qualification, session policy and Gateway origin effects stay open.
Snapshot performs no I/O and cannot detect unread FIN/close-notify/GOAWAY; known
local revocation/expiry checks are not a remote-liveness or H2 idle qualification.
Capture->Network direction remains; no Network dependency on Capture is added.

## Authored verification and missing gates

Eight pure Network cases cover DATA provenance flags, each missing pseudo-field,
duplicate singletons, request mismatch, Host/scheme contradiction, invalid body
length, invalid encoding/control/budgets and unknown duplicate order. Four private
Capture predicate tests cover actual-vs-offered ALPN, missing/unknown/incomplete/
reset stream, trailers/count refusal and the non-authoritative pure happy case.
Two compile-fail documentation fixtures forbid DATA conversion and private view
field construction. All14 are authored, UNRUN; none constructs native proof.

The existing five-case hosted fixture's H2 case is extended to exercise the
actual same-owner view, mismatch refusal and cancellation of an already-borrowed
view. Its denominator remains5, but this changed fixture is UNRUN. Previous5
or72results do not qualify these new files. Standalone rustfmt passed on private
authored files; no compiler/Clippy/native/hosted/Docker execution occurred.

Runtime received the concrete API/coverage contract for review. Root must apply
only after exact preimage checks, then allocate coherent scoped check, strict
Clippy,12pure tests and2negative doctests; actual five-case/socket execution is
a separate explicit allocation. No fake environment or DATA-to-provider proof
may replace those gates. No score, threshold, weight, hard rule or dependency change.
