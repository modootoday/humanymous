# Pages and standards integration

Owner: docs / pages-current-runtime. Scope includes public standards drafts and
standards-governance. No external submission or publication is authorized.

## Root workspace delta requested

Add `docs/tooling` to workspace.members. Package/binary `humanymous-pages` uses
workspace version/edition/rust-version/license/repository and lints. Direct
dependencies reuse workspace serde, serde_json and sha2; add exact compatible
`pulldown-cmark = "0.13.6"`, `scraper = "0.29.0"`, `regex = "1.12"`,
`url = "2.5"` and `percent-encoding = "2.3"` in this package. Root owns the
common lock update. No permanent detached workspace or second committed lock.

Vector delta: reuse humanymous-integrity (path ../../crates/integrity,
version0.1.0) and workspace hex. `vectors --fixtures FILE` reuses the actual
adapter parser. `vectors-export --output NEW_FILE` explicitly authors synthetic
byte fixtures and refuses overwrite; CI never calls it or regenerates expectations.
These vectors do not establish Draft v0 conformance or independent interoperability.

## CI replacement contract

Approved integration window added direct dependency `ureq = "=3.5.1"`, already
present in the shared lock for delivery. Root coordinates the new package edge.
The Rust-only hosted browser pilot will use W3C WebDriver structured HTTP
commands, with no authored JavaScript or execute-script payloads. It requires
an independently started local upstream driver and a current static site server;
neither is launched by pure library tests. Browser/session deletion failures
must fail the report. Keyboard skip focus and measured CSS/geometry checks do
not establish screen-reader, contrast, zoom or full WCAG qualification.

Prepared command contract after that dependency/module integration:

```text
humanymous-pages browser --source docs --site ABSOLUTE --baseurl /humanymous --webdriver http://127.0.0.1:4444/ --base http://127.0.0.1:8080/humanymous/
```

For apex, baseurl is empty and base ends in `/`; for repository mode they are
`/humanymous` and `http://127.0.0.1:8080/humanymous/`. The caller must serve the
exact checked output and keep its driver private. Every served file is compared
with local artifact bytes before/after; all generated HTML is tested twice.
Current source is linked through `pub mod browser` and browser_command dispatch
in main. Root measured locked offline26lib+1CLI passes and all-target Clippy;
the earlier composite Clippy failure in capture remains recorded separately.
The subsequent literal dot-segment driver-ID refusal fix passed a fresh locked
offline library26 run, all-target check and all-target Clippy with unchanged
before/after source and Cargo manifest/lock hashes. The separate CLI1 result
belongs to its preceding checkpoint; it was compiled/checked, not rerun here.
Actual browser execution remains NOT_RUN. Hosted execution must
capture actual report and caller service cleanup separately.

From repository root after integration:

```text
cargo test --locked -p humanymous-pages
cargo run --locked -p humanymous-pages -- source --source docs
cargo run --locked -p humanymous-pages -- validate --source docs --destination ABSOLUTE --baseurl VALUE
```

Run validate twice with empty and /humanymous baseurls. It requires the installed
frozen Ruby bundle, obtains and verifies the pinned v1.5.0 theme source, builds
with Jekyll, runs the upstream Ruby theme checker, then runs Rust output checks.
Ruby/Jekyll/theme remain upstream pinned build dependencies, not new authored
foreign-language implementation. No authored shell/script fragments are spawned.
New source/built/pilot tests are Rust. Legacy Python/shell validator files are
retired after parity and actual two-baseurl execution; CI calls the Rust binary.
Dependency audit and protected production deployment remain separate gates.

## Current boundaries

`vectors-i18n-export --output NEWFILE` explicitly authors a separate ten-case
synthetic corpus and refuses overwrite. The normal read-only `vectors --fixtures`
command validates it with the actual shared parser. The original24-case fixture
is unchanged. This covers parser byte boundaries, not generic wire or HTTP
transport qualification; no dependency or common-manifest delta is required.

`cargo run --locked -p humanymous-pages -- review-delta --root .` compares
the existing authored receipt with a stable source snapshot and reports added,
removed and exact-working-byte changes. It never writes receipts or accepts a
review. Snapshot schema 2 separately reports index paths missing from the
working tree; a file disappearing/reappearing during capture fails the run.
Canonical LF or index-byte equality never substitutes for exact working-byte
comparison against the receipt. This command is author triage, not the CI
docs-review gate and not a release qualification.

Successful `validate` reports `build_input_sha256` only after checking the same
local source-input commitment before and after the actual build and checks.
Configuration, navigation, Sass tokens and binary assets are included. Tooling,
review metadata, installed dependencies and known Jekyll/Sass caches are excluded.
`source`/`built` alone do not emit this commitment or claim a fresh build.

Source review is read-only in CI and must be renewed by the author for actual
governed changes. It does not qualify native/browser/Docker examples by itself.
Runtime, collector, CLI and browser public claims are being checked against their
actual sources and source-bound reports. This file is excluded from Pages output.
Root remains the owner of common Cargo/lock/toolchain and the overall roadmap.

## Current measured checkpoint (not final acceptance)

Root integrated the shared-integrity/hex/ureq edges. Windows locked offline
library26 plus CLI1 pure tests passed with zero failed/ignored/filtered, along
with all-target Clippy. These checks do not launch WebDriver or a browser. The authored
synthetic corpus has24cases:4parser-accepted/20rejected/errors0, explicitly no
Draft v0 conformance. The first real Rust/Linux+Ruby3.3.13 apex build passed79
pages; later85-page apex also passed. Historical hosted63f69ae built88 at both
baseurls with no validation errors; its freshness gate failed and current docs/
checker have changed since. Current rendering/browser/freshness gates and exact
source snapshots are tracked separately in the Pages plan/evidence.

Snapshot command: `cargo run --locked -p humanymous-pages -- snapshot --root .`.
It exports hashes only, separating exact working, canonical LF and actual Git
index blob bytes, marks documentation_review_accepted=false and refuses source,
HEAD/index/inventory changes during capture. This is author-review input, not
an automatic review acceptance or a CI refresh command.

Experimental parser profiles do not establish deployment qualification. Current
source independently provisions Node, nginx, Python ASGI/WSGI, Ruby Rack, Apache
or ASP.NET Core (Node remains default); these are parser profiles, not qualified
host integrations. Required RIT/collector/raw TLS/H2 remain unavailable in that IPC
service. The separate loopback runtime has native and explicit browser RIT
profiles. Native reserves/strips Authorization; working BrowserAuthOnly preserves
application Authorization and inserts status-bound consumed-counter ACKs. These
changed browser paths still need actual execution, not inherited historical
component results. Public references distinguish source wiring from qualification.
