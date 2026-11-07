---
layout: doc
title: Laboratory browser and native integration boundaries
description: Actual native UI implementation versus unqualified framework and deployment support.
permalink: /reference/native-browser.html
reviewed: 2026-11-07
---
# Laboratory browser and native integration boundaries

**Diátaxis quadrant:** Reference. **Audience:** native application integrators and evaluators.

The Windows laboratory prototype combines an owned Meari window with an actual
CEF-rendered child. It has native tab/address UI, window controls and bounded
resize behavior. Its overall browser readiness remains false; a rendered page
or native component test does not qualify external navigation, observation
receipts, automation attribution or enforcement.

The current prototype has a script-free, app-owned `humanymous://newtab/`
document and a narrow internal diagnostic allowlist: `chrome://version/`,
`chrome://gpu/`, `chrome://credits/`, `chrome://license/` and
`chrome://sandbox/`. Arbitrary external browsing is denied. Admission to that
allowlist does not prove every engine page is available: native reports must
distinguish accepted navigation, load completion and observed content identity.
An internal sandbox page is diagnostic information, not a complete OS sandbox
attestation. Raw diagnostic paths/device data belong in private evidence.

Native controls submit address/home/reload intents through the local route
allowlist. Address Enter handling consults the IME guard, address focus and key
repeat; focus loss or disabled IME cancels composition state. The controls stop
dispatching after revocation/closure and project address changes from committed
navigation generations. These source checks are separate from actual keyboard
delivery, candidate-window placement, screen-reader announcements and proof that
a requested CEF page completed loading.

Sandboxing remains required. Do not add security-disabling flags to make a
prototype or test pass. Native source, CEF distribution, OS/architecture and
packaged artifact identity all matter. Specific desktop/DPI observations must
not be transferred to an untested revision or operating system.

The workspace browser has no default features; its executable requires the
explicit `native-ui` feature. A default package check does not build or exercise
the native window. Meari dependencies are pinned to one source revision, while
the separate Windows CEF bootstrap pins its own upstream revision and enables
the sandbox feature. These dependency declarations are not packaged-artifact
or runtime sandbox evidence.

The bootstrap source rejects a missing sandbox pointer and listed weakening or
remote-debugging switches before loading the desktop DLL. It calls CEF process
dispatch before application I/O and UI loading, then loads a fixed sibling
`humanymous_cef_desktop.dll` only in the browser process. This assumes the matching
pinned bootstrap ABI and a trusted private bundle; the fixed filename alone
does not authenticate DLL bytes. The feature-gated native refusal fixture checks
four switch forms, exit code and unchanged synthetic-profile directory names.
It does not establish comprehensive switch coverage, process isolation or
current native execution. Preserve those as separate qualification gates.

The historical Windows integration receipt for browser source `a924603` records
seven native unit tests and one explicitly enabled refusal test covering four
switch forms, with zero ignored tests. That narrower run did not test GUI
keyboard, IME or assistive technology. Later process-local scheme-registration
changes have separate source records with native compilation, initialization
and missing-registration failure execution still unverified. Do not transfer
the earlier passing receipt to those changed paths or describe all browser
execution as absent.

The current renderer probe runs only after that process observes successful
owner-scheme registration and receives an explicit positive probe-owner value.
It attempts only generated canary paths beside the executable and sends numeric
read/write outcomes, not file contents. Missing callbacks, registration, frame
or message delivery produce no observation. They must remain unknown rather
than becoming a successful denial. A missing-file error is also distinct from
access denied; neither result qualifies every renderer, utility process,
filesystem path or network boundary.

The desktop pilot contract separately requires two feature-enabled process
cases: positive internal navigation and cancellation before creation. They need
fresh, separately provisioned bundles; a missing bundle or an omitted feature
is not a passing zero-test result. The source bounds capture to 90 seconds and
1 MiB and the private report to 8 MiB. Its process supervisor reaps the direct
child; this does not attest cleanup of all descendants. Expected navigation and
renderer-probe counts in the contract are required observations, not executed
results or assistive-technology coverage.

The process-owning prototype DLL is not the planned embeddable SDK. SDK lifecycle,
versioned ABI, consumer ownership/unload and framework compatibility remain
separate responsibilities. The `humanymous-native init` planner can describe a
requested integration while reporting `not_implemented` and `qualified: false`;
those fields are deliberate and must not be treated as successful SDK activation.

The browser does not imply DRM/CDM availability or Chrome feature parity.
Missing media capability, an accessibility API or a similar user-agent string
does not by itself establish automation or malicious intent. See
[native planning tutorial](../tutorials/quickstart-monitor-mode.md) and
[implementation status](production-vs-reference.md).
