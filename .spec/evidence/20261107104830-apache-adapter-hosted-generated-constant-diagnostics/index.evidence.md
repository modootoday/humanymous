---
id: evidence:root/20261107104830
title: Apache adapter hosted generated constant diagnostics
status: active
created: 2026-11-07T10:48:30Z
updated: 2026-11-07T10:48:57Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 52e7241bca6addc8e5537d2cd4e5d20f3ae26eab0fe3a4d808adb3d45693f6c4, bytes: 1827, store: repo}
---

# Apache adapter hosted generated constant diagnostics

## What was measured

Actual standard GitHub run42365027138, component job128381171717, source
d2a20dd5b9b3793cd7b96f69ef880b55cce73854. Both pure unit groups passed9and10
with zero failures/ignored/filtered, and the pilot check plus both warnings-denied
Clippy preflight steps passed. Actual Docker build exited1 after228.582seconds.
The safe projection contained no Rust E codes and identified host.rs pattern
locations748:13,749:13and750:13, where generated mixed-case constants were used.

Code review infers a foreign constant-pattern naming diagnostic at these exact
locations. The scoped candidate uses uppercase typed Rust aliases of the same
generated values; it changes no read-mode mapping, fail-closed predicate or lint
level. Only the next actual selected-httpd build can validate this candidate.
The first E0282 diagnostic was not repeated in this projection. No stronger
claim about an unexported intermediate build artifact follows from that absence.

The coordinator preserved the verified archive source and manifest identity on
this failure. Its owned cleanup and the separate always-cleanup step passed.
Raw job logs were captured privately. No private bundle or automatic artifact
upload occurred.

## How to read it

No final image, ABI/module artifact identity or scenario report was observed.
All45required scenarios remain NOT_RUN, observed counts are null, and
fullProductSuccess is false. This exact failure is retained independently from
the prior run and any subsequent correction.
