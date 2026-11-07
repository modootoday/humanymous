---
id: change:root/20261107073933
title: Browser UI navigation lifecycle and strict probe source checkpoint
status: active
created: 2026-11-07T07:39:33Z
updated: 2026-11-07T07:39:33Z
rel:
  depends_on: [sot:root/native-embedding, plan:root/20261107042614, sot:root/browser-ui, sot:root/rust-migration, plan:root/20261107002014, plan:root/20261107033358, plan:root/20261107041001, plan:root/20261107050329, plan:root/20261107071648]
---

# Native navigation source checkpoint

Accepted UI-25..27 in browser-ui0.6 centralize pure navigation ticket/terminal
rules and ordered7-step qualification. Native adapters now refuse stale captured
work, closed/revoked input/observer/canary results and out-of-order cross-route
callbacks. New requests supersede only after acceptance; truncation is counted.
Task/frame/status/source/timeout failures remain terminal, and independent probe
failures are collected. Probe failure is separate from cleanup failure: CEF
shuts down after real child/parent cleanup before a failed probe result returns.

Report revision2 is an additive contract with CI/native-runner owners: require
schema2, probe requested/finished/passed, observed7, dropped0 and real cleanup.
Combined cancel-before-created/probe negative case is specified and not_run.
Central policy remains program-roadmap/delivery; no workflow, dispatch, shared
Cargo/lock/toolchain, runtime, collector or product-lab source was edited.
Root owns local Cargo assignment and remote source closure. The former misc
ownership was normally released after root introduced formal browser-ui lane.

The pure IME policy is available without native-ui dependencies and resets canceled
composition. Address/control bounds precede URL parse and bounded query decode.
No authored foreign application/test code, scoring/policy-threshold change,
sandbox weakening, global hooks or third-party probing is introduced.

Evidence:root/20261107073932 is source-only: formatter, explicit no-local scoped
Specfold and whitespace checks pass. Units/Clippy await assigned Cargo slot;
native compilation/process/GUI/IME/AT/GPU/DPI await hosted/suitable environment.
Zero tests/probes were executed in that evidence. Goal remains active. Same-URL
CEF callback attribution, full sandbox/BrowserReady/external navigation and all
previous native qualification gaps remain open. Older native/remote green is
not evidence for this changed source.

Later root assignment completed default pure23/23 and same-scope Clippy after
retained first missing-field-rustdoc failure and corrected rerun. Evidence
pure-results.json preserves both source versions/results; native/hosted not_run
and active goal remain unchanged. No old green is applied to new native code.


## Root-expanded native scaffold consumer ownership

Root explicitly added crates/native-cli to the browser owner's exclusive scope;
operational crates/cli stays separate. Its only library Directory helper consumer
is Windows storage tests, so the helper module now uses cfg(all(test,windows)).
The independent integration-test helper remains unchanged on supported platforms.
Hosted Linux dead_code3 from42348518353/source63f69ae is retained as a failure;
new Linux verification is not_run. No local Cargo was started after root opened
the shared graph window. No test/gate removal or warning suppression occurred.

All new owned documentation/comments are English. Existing Korean IME fixtures
and precisely measured characters in historical receipts are intentionally
preserved, as are immutable raw logs and their hashes. A bounded scan of current
owned source/prose found only those functional/historical multilingual values;
no authored Korean prose/comments required translation in the touched scope.


## UI-28 source follow-up after c4b58be

Registration rejection previously only logged. Actual process-local acceptance
now gates desktop owner factory and renderer canary work; missing signal is not
success. This changes no IPC/report schema and grants no all-process/sandbox
attestation. Registry atomics are application-DLL/process-local. Native compile,
positive/missing-registration runtime checks are hosted not_run. Core23 proof
is unchanged and does not validate this native delta; formatter alone passes.
