---
id: evidence:root/20261107205600
title: Consolidated platform feature and boundary validation
status: active
created: 2026-11-07T20:56:00Z
updated: 2026-11-07T20:56:00Z
rel:
  depends_on: [sot:root/structural-development, sot:root/program-roadmap, plan:root/20261107203000]
files:
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
