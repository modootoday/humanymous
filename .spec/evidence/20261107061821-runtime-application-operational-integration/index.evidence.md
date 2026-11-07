---
id: evidence:root/20261107061821
title: Runtime application operational integration
status: active
created: 2026-11-07T06:18:21Z
updated: 2026-11-07T07:50:56Z
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/runtime-application]
files:
  - {path: status.json, sha256: 39b19d937cd89fb63e657d971d329e884b311019118eb74a19a1667705801b11, bytes: 1778, store: repo}
  - {path: hosted-42348518353-runtime-summary.json, sha256: 33619f43f669308c5cc0a03834a6cd48eba5a993f1fbc894aae717af4ec8cfec, bytes: 1128, store: repo}
  - {path: source-commitments.json, sha256: 5698a9b8b74a1f1b83c796d949379e14cb7db6c1513d69f92230ba4f523d4e52, bytes: 5693, store: repo}
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