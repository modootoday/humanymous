---
id: evidence:root/20261107061821
title: Runtime application operational integration
status: active
created: 2026-11-07T06:18:21Z
updated: 2026-11-07T09:05:20Z
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/runtime-application]
files:
  - {path: hosted-42348518353-runtime-summary.json, sha256: 33619f43f669308c5cc0a03834a6cd48eba5a993f1fbc894aae717af4ec8cfec, bytes: 1128, store: repo}
  - {path: cli-dependency-static-observation.json, sha256: b0b28c5e59e9cd8e4ee51c00f1da123674e679676ce6788b1912589034b2c86e, bytes: 868, store: repo}
  - {path: status.json, sha256: 110d3d4d692b78e270975802d9c306ec5d4219eb35ac65ed66a07106041b2ce3, bytes: 2564, store: repo}
  - {path: source-commitments.json, sha256: d586934a61231ff968a5aa67440678b249ae55f15aa38e9dfcf6d8ade8a12533, bytes: 7690, store: repo}
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