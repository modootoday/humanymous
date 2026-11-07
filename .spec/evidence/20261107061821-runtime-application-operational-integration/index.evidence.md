---
id: evidence:root/20261107061821
title: Runtime application operational integration
status: active
created: 2026-11-07T06:18:21Z
updated: 2026-11-07T09:45:47Z
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/runtime-application]
files:
  - {path: hosted-42348518353-runtime-summary.json, sha256: 33619f43f669308c5cc0a03834a6cd48eba5a993f1fbc894aae717af4ec8cfec, bytes: 1128, store: repo}
  - {path: cli-dependency-static-observation.json, sha256: b0b28c5e59e9cd8e4ee51c00f1da123674e679676ce6788b1912589034b2c86e, bytes: 868, store: repo}
  - {path: hosted-42355168729-runtime-summary.json, sha256: 6f131ab8738dd6e2976b47a5afa7403736e349bdbf82d13a1a279ef332c7617a, bytes: 4941, store: repo}
  - {path: status.json, sha256: 2ca2945b34584b6690b9a570fff5f517eafb441e0668627e6d1264b0ae8600f6, bytes: 3349, store: repo}
  - {path: source-commitments.json, sha256: 4e800db36ca48d2c8a5d4aadefebf0c33a5997026fc42aa2890553c3f6727c50, bytes: 8815, store: repo}
---

# Runtime application operational integration

## What was measured

This is the follow-through contract/evidence cover, not a successful execution
receipt. Contracts R01-R17 are accepted/proposed as individually recorded in
plan:root/20261107061820 and sot:root/runtime-application v0.3. New runtime
implementation is unvalidated; actual baseline test count0 due to lock refusal.

R02 server source and two shared-client native fixture tests are now authored.
Current-source tests/runtime/browser/Docker remain NOT_RUN. An intermediate
production-only check passed before final edits and has no exact manifest; it is
not attributed to this checkpoint. Only owned formatting and public-only spec
syntax checks are current (0errors/1unrelated existing binary warning).

## How to read it

Heavy validation is held for user-requested memory triage. Do not infer browser,
human/keyboard/AT, capture, management, service or full product qualification from
this document's existence or predecessor component results. Raw history belongs
to private evidence:root/20261107061820; public attachments will contain only
sanitized outcomes/metadata/hash commitments with LF/stored-byte checks.

Historical hosted evidence recovered2026-11-07: run42348518353 at exact source
63f69ae9de703939b7b9091e2f215b12bea265de completed FAILURE,16jobs:3success/13failure.
MSRV job128338631933 explicitly executed runtime15tests with15passed/0failed/
0ignored/0filtered, including the native shared-client ACK fixture. That source
predates later management/stream/admission/shared-owner work. These are native
fixture/component results with client-asserted report and mock audit, not actual
browser/human or full witnessed product evidence. Latest source remains NOT_RUN.
Private captured CLI stdout (not original HTTP response bytes) is attached under
evidence:root/20261107061820; sanitized exact-source summary is attached under
public evidence:root/20261107061821. Other failed native probe details remain
unrecovered; no success is inferred. No rerun/dispatch occurred.
Historical813 MSRV job128359709010 actually passed runtime library41 and native
process1 with0failed/ignored/filtered; runtime main0 excluded. Overall run failed.
The attached sanitized summary binds exact source/tree, command, names and private
CLI-capture hash. These default workspace-feature predecessor components do not
qualify current TLS/operator/control source, actual browser or full product.
Current source still awaits Root allocation after benchmark quiet window.