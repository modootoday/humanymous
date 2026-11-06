# Mobile research provenance

Observed 2026-11-07 KST. Three read-only subagents investigated official Android/
Appium/Genymobile, Apple/Maestro, and provider attestation/device-farm sources.
Root additionally verified Android Studio Agent Mode and pinned Mobile Next MCP
source metadata. Source links and scope limits are in mobile-agent-tools.sot.md.

All device tests are not_run. No tool installation, SDK provisioning, device
connection, permission/signing change, provider account operation or external
target probe occurred. Remote capability documentation is not runtime evidence.

Version observations: UiAutomator2 v9.0.1 and scrcpy v5.0.1 were observed by the
Android researcher; not installed or artifact-attested. Mobile MCP main snapshot
57efc27d96971f96caba69b62665fa057ebc65ac is pinned independently. Appium latest
non-macOS-host documentation was directly read, dated 2026-10-28; an indexed
versioned /12.16 URL returned 404 and was not used as release evidence.

Critique focus: real hardware is automatable, attestation is not human proof,
accessibility absence/presence cannot classify intent, and setup failure is not
defense success. Documentation-only Specfold checks validate records, not devices.
