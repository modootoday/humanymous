# Browser research provenance

Observed 2026-11-07 KST. Three read-only researchers used official CEF, Rust
binding, Chromium, Electron, Tauri and W3C sources. Commit metadata is attached;
source links, proposal states and limitations are in chromium-lab-browser.sot.md.
Independent security critique found no P0/P1 blocker for research-only canon;
native update/licensing and product-context wording feedback was reflected.

All build, sandbox, UI, browser and Cross-OS qualification cases are not_run.
Electron was investigated as a comparison and excluded from Rust-only authored
application code. OpenAI Browser documentation confirms product behavior and
CDP developer access, not a particular embedding library. Owl reconstruction
was withdrawn by the user and is not part of the accepted plan.
