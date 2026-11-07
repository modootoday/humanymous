---
id: sot:root/pages-implementation
title: Authored Pages implementation and verification
status: active
created: 2026-11-06T22:35:00Z
updated: 2026-11-06T22:35:00Z
verify:
  - cargo test --locked -p humanymous-pages
  - cargo run --locked -p humanymous-pages -- source --source docs
rel:
  governs: [docs/**, scripts/docsgen/**, .gitattributes]
  depends_on: [sot:root/delivery, sot:root/standards-governance, sot:root/automation-evidence]
---

# Authored Pages implementation and verification

## Rule

Public content is English authored Markdown. No script generates page bodies.
Reader source rejects Unicode control characters except tab/newline after CRLF
normalization; Unicode text and format marks are not a substitute target for this
check. It prevents accidental escaped-shell control bytes, not all visual spoofing.
All reader pages declare doc layout, stable permalink, reviewed date, Diátaxis
purpose and audience. The seven standards research pages retain their authored
substance, project-draft maturity and explicit no-submission/no-adoption status.
Private identifiers, operational secrets and personal traces do not belong in
the site. AGENTS.md, scripts and review metadata are excluded from output.

The source checker also validates the deliberately constrained authored
`_data/nav/main.yml` grammar: root flow-map links or named groups with exactly
indented items, simple unquoted titles and URLs matching authored permalinks.
Missing menus, malformed indentation, unknown targets and empty groups reject.
This is a local authoring contract, not a general YAML parser or rendered-theme
validation; expanding menu syntax requires explicit checker/tests review.
Rendered link checks resolve relative, absolute and protocol-relative URLs
against the configured canonical origin. Same-origin targets all require local
artifact, fragment and exact baseurl containment checks. Different-origin links
are not fetched or certified by this offline checker. Output checks are not a
browser, CSS/layout, keyboard or assistive-technology execution result.
Static accessibility checks follow the pinned theme's actual layout contract:
one nonempty document title and a labelled `a.mt-skip` fragment link to an
existing `tabindex=-1` target inside main, without explicit hidden/aria-hidden
ancestors. These are HTML structure checks only, not computed style, actual
focus movement, keyboard reachability, screen-reader output or WCAG conformance.
A later browser pilot must record its own cases and actual outcomes separately.
### Theme and build authority

Source-review triage uses `humanymous-pages review-delta --root .`, a read-only
comparison with docs/reviews.json. Snapshot schema2 preserves exact working,
canonical LF and Git index byte identities and separately reports index-listed
files missing from the working tree. Missing paths are rechecked for races.
Triage uses Git-visible governed paths and is not a replacement for the CI
filesystem inventory or substantive authored review. It cannot renew receipts.
Authors may manually renew only substantively inspected source subsets with
exact working-byte hashes, specific reasoning and reader-page mappings. A subset
does not accept the complete inventory or prove required named checks executed.
Rendered-site exclusion is not confidentiality for committed review metadata.
Real-build validation commits exact local input paths and bytes before and
after Jekyll/theme/output checking. It includes navigation, configuration,
Sass tokens and binary assets; verifier/tooling, review metadata, installed
dependencies and known build caches are excluded. Changes refuse success.
Successful validate output reports build_input_sha256. It binds local inputs,
not all external dependency bytes or product release qualification.

`docs/_config.yml` uses remote_theme pinned to
`modootoday/jekyll-theme-modootoday@ba2b23f0e1639f92ccefbb882f43092bfe5b16b7`,
verified against v1.5.0. Preserve CNAME humanymous.net, apex url and empty
production baseurl. Validate a second actual build with /humanymous baseurl.
The site uses the upstream doc layout and native `_data/nav/main.yml`/footer.
No custom layout shadow or legacy OG/LLM prose generator remains in the build.

Build via Actions using Ruby 3.3.13 and locked Jekyll 4.4.1 with Pages-compatible
plugins, not classic GitHub's github-pages 232 dependency bundle. The pinned
theme supports this Jekyll path. `jekyll-remote-theme` 0.4.3 constrained RubyZip
below the fixed release and failed the advisory gate (CVE-2026-85396). Use
remote-theme 0.6.3 and RubyZip >=3.4,<4; the lock currently selects 3.7.0.
Never suppress that advisory or loosen frozen installation in CI.

The accepted replacement CI interface is
`cargo run --locked -p humanymous-pages -- validate --source docs --destination ABSOLUTE --baseurl VALUE`,
run from repository root. The package lives at docs/tooling and shares the root
Cargo workspace/lock/toolchain. New implementation/tests are Rust1.99.1,
edition2024; no authored script strings or foreign-language helpers are used.
It requires a frozen installed Ruby bundle, validates
source, verifies the v1.5.0 checkout SHA, builds using the configured remote SHA,
runs that exact theme's checker with --strict, and validates output. Neither
it nor the source checker writes authored content/review evidence. It does not
publish. `bundle exec bundle-audit check --update` is a separate mandatory gate.
The upstream pinned Ruby/Jekyll/theme checker remains an external dependency,
executed through Command argument arrays. It is not newly authored product code.
Legacy Python/shell wrappers are retired after actual two-baseurl parity; the CI
workflow now invokes Rust. Exact owner contract is docs/INTEGRATION.md.

Commands source/built are read-only. validate permits only an explicit absolute
dedicated output outside/away from source ancestors, rejects symlink output
escapes, verifies the exact upstream theme tag and invokes real Jekyll/checker.
Unknown/duplicate CLI arguments and invalid input fail. Output errors are not
silently converted into a pass; there is no publication command.
Markdown links are parsed with the maintained CommonMark parser and HTML with
the maintained DOM parser. The checker accounts for reference links/images,
percent-encoded anchors, canonical baseurl and named index-suffix files.

The site checker requires all source links to exist within docs, explicit page
metadata, unique URLs, draft status and no obvious private identifier/key patterns.
Output checks require every authored page, one main/h1, English lang, exact
baseurl-aware canonical URL, local target/anchor integrity, alt attributes,
unique IDs and exclusion of retired/private surfaces. This is bounded static
validation, not a complete secret scan or accessibility certification.

Public profile requirement IDs are allowed when their scope is defined and each
has a descriptive heading and an individual validation link. They are distinct
from private specification IDs and internal priorities. Keep review checklists
explicitly informative and reserve uppercase BCP 14 language for declared
candidate normative content. Expand unfamiliar abbreviations at first use or
link a public glossary. Configure Sass sourcemap: never; the output checker
rejects any .map debug artifact, including future accidental reintroduction.

### Tokens

Only the three supported override maps exist in
`docs/_sass/modootoday/_site.scss`. Retain baseline background/card/text and
status palette, system fonts, 820px measure and 10/14px radii. Light links use
the accessible #08695e variant; primary foreground uses #04120f rather than
white on teal. Dark muted foreground is raised from #808fa3 to #9aa9bc for
readability. Accent/status colors are not sole status communication. Record
actual text contrast and browser light/dark/mobile review before acceptance.

### Ownership and freshness

Pages owner: docs/**, retired scripts/docsgen/**, this document, own pages plan
and private pages evidence. CI owner: .github/workflows/**, scripts/ci/** and
delivery canon; do not edit those paths from this lane.

The read-only Rust checker lives in `scripts/ci/docs-review`, owned by CI.
`docs/reviews.json` schema 1 records each source, SHA-256, reviewer, substantive
notes and reader pages for every file under crates/adapters/web/build plus
root Cargo.toml/Cargo.lock. It is source review metadata, excluded from the
public artifact. Changed, added or removed governed source needs renewed review;
do not automatically refresh it in a release job. The complete inventory and
exact source mappings are required; a placeholder is a release blocker.

The continuous user goal also owns standards-governance and public standards
documentation. CI owns freshness-gate implementation, root owns common Cargo/
lock/toolchain, producer owners own runtime APIs. Pages receives interface
changes, reads actual current source and explicitly reviews affected prose.
No gate automatically refreshes reviewer notes/digests. Dirty working-source,
canonical LF and Git stored bytes must be distinct evidence fields.

Freshness must be portable across supported OS checkouts. A raw digest of CRLF
Windows text is not equivalent to the LF Git object checked out on Linux.
The user selected root `.gitattributes`: `* text=auto eol=lf` plus explicit
binary suffix rules. The source ledger now hashes canonical LF text and exact
binary bytes. A canonical LF export passes the unchanged CI-owned verifier.
Existing actively edited shared files are not rewritten by this lane; their
owning session must refresh/normalize its working text without discarding edits.
Subsequent checkouts enforce LF on Windows as well as Unix. Actual cross-OS CI
remains a separate execution gate. The original 53-path mismatch is recorded.

Reader mapping: Rust detection/network/model responsibilities map to computation,
capability and model-limit pages; integrity to request-integrity/origin-contract;
audit to audit-continuity/erasure/security-evidence; adapters to integrator and
application-compatibility pages; existing web and Go-based Docker builds to
status/requirements/migration boundaries. Current docs explicitly avoid
claiming a runnable Rust replacement or supported release tuples. Same-commit
native/browser/Docker pilots remain separate gates, even if review digests pass.

### Cleanup and publication

Retire the custom default layout, global llms.txt, OG/LLM twins, screenshot/demo
media, old CSS and prose/SEO/screenshot generators. Preserve only used brand
assets. Do not rewrite published history. Source cleanup and checked output
are separate checks. No external submission, public deployment, registry publish,
main merge or tag is authorized by this lane's task.

## Why

The user requested a complete authored rewrite, pinned supported theme, honest
Rust migration status, private/public separation, maintainable locked build and
real two-baseurl validation. The previous site shadowed the theme and generated
secondary prose/assets. Tests of docs must not imply runtime qualification.

## Verification

The output checker validates every rendered HTML file, including generated
pages without authored Markdown counterparts. Authored-page presence is checked
separately. Directory URLs and explicit `index.html` links share anchor lookup;
missing fragments cannot bypass validation through the explicit filename.

Execution and remaining gates: [[plan:root/20261106223501]]. Private raw logs:
`.spec.local/root/evidence/20261106223502-pages-build/`. The inherited standards pilot
evidence is updated only with actual run outcomes, never placeholder success.
