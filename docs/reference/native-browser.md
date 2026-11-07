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

Sandboxing remains required. Do not add security-disabling flags to make a
prototype or test pass. Native source, CEF distribution, OS/architecture and
packaged artifact identity all matter. Specific desktop/DPI observations must
not be transferred to an untested revision or operating system.

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
