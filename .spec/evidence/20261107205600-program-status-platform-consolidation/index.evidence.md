---
id: evidence:root/20261107205600
title: Consolidated platform feature and boundary validation
status: active
created: 2026-11-07T20:56:00Z
updated: 2026-11-07T20:56:00Z
rel:
  depends_on: [sot:root/structural-development, sot:root/program-roadmap, plan:root/20261107203000]
files:
  - {path: fixture-selector-rename.json, sha256: 658445fa24c7a7eca37814ac257f094037c6afe50a1bb71bf8c5161fb1094ea3, bytes: 570, store: repo}
  - {path: browser-edc-dispatch.json, sha256: 18603c7b2ac3f1b7288b1ea144b4a28b18360ac042ce5267dc309e9caa0818c3, bytes: 888, store: repo}
  - {path: branch-sync.json, sha256: 441924321e7a62a5bf363e7b894ff49fd9ec889f9f84cb8f6a3943100e3f964c, bytes: 1353, store: repo}
  - {path: summary.json, sha256: 540583b7eb7f34e91b75c17cc6d89afa1601f4fa60dad6495c97ed235c6bd34d, bytes: 16541, store: repo}
---

# Platform slice observation

Two former packages now occupy one platform package with testing and
windows_object contexts. Thirty current package files include the26 relocated
non-manifest inputs, a merged manifest, a new composition root and package
README/license. The workspace resolves34 members, not the final eight.

The testing-only executable lists55 tests. Exactly41 pure tests passed with
zero failures or ignored cases and14 explicitly filtered cases: seven controlled
capture/barrier parents, six legacy process parents and one subprocess entry.
These were not bridge tests. Core145 unit plus17 integration cases passed using
the new platform development dependency;179 selected inputs stayed unchanged.
The separate Windows-only bridge executable also passed four pure wire/path/
capacity cases, with no native query execution or filtered cases.

Windows all-target/all-feature checking and strict platform/core Clippy passed.
The bridge documentation has one positive import test and one compile-failure
test. Three same-crate source probes used the real context modules: the control
passed; forging observation fields failed with E0451 and reaching Worker failed
at its private parent module with E0603. Thirty-six selected source, extern and
compiler inputs stayed unchanged; this is not full toolchain/loader custody.

Nine isolated API checks verify four positive Windows feature combinations and
five absent-API failures, including both absent default modules and the Windows
module's absence on WASM. WASM is cross-compilation, not another OS runtime test.
Normal offline locked Cargo packaging succeeded and verified the extracted
default library in a33-file archive. This does not establish namespace ownership,
all-profile extracted consumers, native execution, product qualification or
publication. Preserve the recorded pre-compiler orchestration failures and
formatter corrections independently of these successful bounded checks.

Consumer source updates are coordinated, but comprehensive consumer/workspace,
Cross-OS runtime and actual product gates remain pending. Historical native and
Docker evidence remains tied to its original sources and helper names.

The branch-sync attachment records the atomic fast-forward of two validation refs to1f87d310. It is a discovery-time record, not a claim that either hosted run passed. Main and the remaining nine validation refs were not updated by that transaction.

A later scanner flagged a static controlled-test selector as a credential pattern. The narrowly renamed function and matching grant string preserve the case identifier, expected outcome and body. Original scanner output and historical receipts remain unchanged; the source rename has formatting validation only here. Actual controlled-process execution and a new-source scanner result remain pending.
