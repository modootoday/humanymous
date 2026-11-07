---
id: sot:root/delivery
title: Rust delivery and Pages contracts
status: active
created: 2026-11-06T20:27:03Z
updated: 2026-11-06T22:06:50Z
verify:
  - python scripts/ci/delivery.py policy
  - python -m unittest discover -s scripts/ci -p "test_*.py"
  - cargo test --locked --manifest-path scripts/ci/docs-review/Cargo.toml
rel:
  governs: [docs/**, .github/workflows/**, scripts/ci/**]
  depends_on: [sot:root/rust-migration]
---

## Collector component validation delta c3389b6

collector-component.yml adds read-only, SHA-pinned Actions validation of actual
Rust WASM tests in the network-none, non-root Docker Firefox component lab.
It uploads image metadata and test output by source SHA and publishes no package
or image. This independent component check does not replace any existing native,
browser or Docker full-delivery gate. Remote run42327415870 was started; its
conclusion remains pending at this observation. Full-runtime qualification and
the final delivery runner requirements remain unchanged.

# Rust delivery and Pages contracts

## Rule

This is the accepted delivery design, not a claim that the workflows, packages
or documentation rewrite are implemented. Replace manual verification with real
workflow/document-contract checks as those components land. The full Rust
migration remains the release prerequisite.

### One validation authority

A reusable unprivileged validation workflow is called by PR/main CI and the
tag release workflow. Every release runs it on its exact tag commit; a historical
green workflow or a skipped Docker job is insufficient. Its required jobs are:

- Linux, macOS and Windows locked workspace builds/tests and native executable
  startup/configuration pilots; use fail-fast false and unique artifact names.
- MSRV compatibility plus stable formatting and warnings-denied Clippy.
- The browser WASM build and real browser integration against Rust.
- Linux-only isolated Docker attack, Gate, Pass, swarm and deployment pilots,
  using Rust images. Preserve topology and human-baseline assertions.
- Fresh dependency/advisory scans, package-content/metadata checks, documentation
  review freshness and theme source/built-site validation.

Validation has contents read only. Permissions are granted separately to crates
OIDC publication, GHCR/signing, GitHub Release creation and Pages deployment.
Publication concurrency never cancels an in-flight release. PRs never publish.
The defense-only bots image is never a public package/image release target.

### crates.io publication

Use strict SemVer tag/version equality, main ancestry, reviewed changelog/docs,
validated package archives and the Cargo dependency graph before any upload.
Publish registry dependencies before dependents, including versioned dev
dependencies needed to execute packaged tests. Never ship repository-relative
test paths or unpublished required dependencies.

Publishable manifests require complete metadata, registry-compatible dependency
versions beside development paths, licenses/readmes and explicit package content.
Current publish=false declarations intentionally prevent premature publication.

Authenticate through the official crates.io OIDC action immediately before
publication, pinned to a verified commit. The reviewed v1.0.6 action commit is
3268eb413133e2960fa4ae0e963ca76380c54cd9; do not assume an example's moving v1
tag exists. Keep short-lived credentials confined to the publishing step and
allow the action's revocation cleanup. A long sequence reauthenticates between
dependency waves. No long-lived token fallback belongs in the release workflow.

The Trusted Publisher binds repository owner/name, exact workflow filename and
the protected release environment. It is configured per crate after ownership
and initial registration. Current crates.io requires an initial non-OIDC upload
before that registration; this is an external account bootstrap prerequisite,
not a capability to fake in YAML. Do not perform it or claim live OIDC success
from packaging dry runs. User-owned account setup is recorded as unverified until
exercised; the subsequent automated release path must use OIDC.

On rerun, distinguish network/auth failures from a verified already-published
version. Check matching validated artifact identity before skipping an immutable
version. Registry publication is not an atomic multi-crate transaction; preserve
honest partial-failure records and resume safely without overwriting history.

Sources: [crates.io Trusted Publishing](https://github.com/rust-lang/crates.io/blob/main/svelte/src/routes/docs/trusted-publishing/%2Bpage.svelte),
[official authentication action](https://github.com/rust-lang/crates-io-auth-action).

### Authored Pages source

The site uses remote_theme, pinned to reviewed v1.5.0 commit
ba2b23f0e1639f92ccefbb882f43092bfe5b16b7. Keep title/repository, the existing
humanymous.net domain/CNAME, empty baseurl, GFM and explicit page front matter.
Default reader pages use the theme doc layout. Remove the overriding legacy
default layout, old generated prose/LLM twins and the docsgen build dependency.
Human/agent authors edit Markdown directly; CI validates rather than generating
documentation content. No scripted replacement prose or stale Go examples.

Use a locked, vulnerability-checked GitHub Pages-compatible Jekyll stack with
jekyll-remote-theme. Run the checker from the same theme version against source
and built HTML, including links/navigation, then inspect light/dark/mobile
renderings. Optional twins are omitted unless their source is authored and
maintained without the prohibited documentation generation flow.

Build and deployment are separate. The protected github-pages deployment needs
the checked site artifact from the same commit and receives only its required
Pages/OIDC permissions. Configure the repository's Pages source for Actions.
Do not publish a preview of an incomplete Rust replacement to the public site.

Sources: [theme install](https://github.com/modootoday/jekyll-theme-modootoday/blob/v1.5.0/docs/guides/install.md),
[Pages workflow requirements](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

### Design token ownership

Port the baseline docs palette, typography, radius and measure through the
supported docs/_sass/modootoday/_site.scss bridge, using only its three override
maps. Do not shadow theme internals or copy the old component stylesheet. Map
semantic roles explicitly, preserving the brand palette; verify actual text
pairings before accepting a palette mapping.

| Baseline role | Theme role | Light | Dark |
| --- | --- | --- | --- |
| bg | background | #eef1f6 | #0a0e14 |
| text | foreground, card-foreground | #14202e | #e4ecf5 |
| panel | card | #fff | #111722 |
| panel-2 | muted, code-background | #f6f8fb | #0d131c |
| muted | muted-foreground | #566579 | #808fa3 |
| border | border | #d9e0ea | #202a38 |
| raise | accent | #fff | #16202c |
| accent | primary, brand, ring | #0e9c8a | #35d0ba |
| accent-ink | primary-foreground | #fff | #04120f |
| allow | tip, status-available | #1f9d43 | #3fb950 |
| challenge | warning, status-partial | #b3811f | #d9a441 |
| deny | caution | #d63a52 | #f0556a |
| info | note | #2b7fe0 | #4a9eff |

Map system font stacks to font-sans/font-mono, 820px content width to measure,
and 10px/14px corners to radius/radius-lg. The link/text-on-primary assignments
require contrast verification; adjust semantic use or a documented contrast
variant rather than silently assuming the old pairings are accessible.
Retire unused faint/border-soft/dim/spacing variants, or keep them as documented
product-owned properties only where a real site component still needs them.
Source: [supported token bridge](https://github.com/modootoday/jekyll-theme-modootoday/blob/v1.5.0/docs/guides/customize.md).

### Release documentation freshness

A read-only Rust verification command checks authored review evidence in .spec:
each shipped command/API/configuration surface and capability has a reader page,
governed source hashes, review notes and executable example/pilot coverage.
Changed governed source requires an explicit renewed documentation review; the
gate never repairs its own evidence or merely accepts any Markdown edit.
Unchanged prose may be retained after a concrete review confirms it still holds.
Run examples/config parsing against Rust and require the same-commit CI pilot
jobs. Missing owners, stale digests, broken navigation, stale Go instructions or
an unexecuted required check block crates/images/releases.

## Why

The user explicitly requested cross-OS builds, OIDC crates.io automation,
release-time documentation review, remote_theme-based Pages and token migration.
The design panel found current path-only/publish=false manifests, broad existing
release permissions, old layout overrides and registry bootstrap prerequisites.
Those findings are requirements above, not hidden implementation assumptions.
The adversarial review additionally forbids stale-SHA authorization, token
fallbacks, vacuous documentation gates and treating registry errors as success.

## Verification

### Workflow implementation boundary (2026-11-07 KST)

The reusable `.github/workflows/rust-validation.yml` now owns mandatory native
Linux/macOS/Windows, user-approved pinned 1.99.1 MSRV/toolchain format/Clippy, actual WASM/browser,
Linux Docker lab, defender image scans, packages, Node/Bun, advisory, authored
documentation review and Pages jobs. The aggregate explicitly rejects any
skipped/failed/cancelled required job. `ci.yml`, `release.yml` and Pages deployment
call this workflow at their own checkout commit. No historical run lookup grants
publication. Frozen Go CI remains a manual oracle and its original release YAML
is inert provenance under scripts/ci, never a shipping path.

`scripts/ci/README.md` fixes the integration interfaces and execution-report
requirements. Runtime reports bind exact source SHA and authored documentation
review SHA-256, require all named cases (including executable documentation
examples), and reject skips/errors. Docker evidence additionally requires Rust,
Docker isolation and immutable image identities. Runner code review and actual
execution remain required; a JSON report alone is not behavioral proof.

The isolated Rust tool at scripts/ci/docs-review is read-only. Its schema-1
docs/reviews.json interface requires exact SHA-256 source inventory, authored
review notes/pages, and fixed command/API/configuration/capability coverage with
named pilot cases. It cannot repair evidence, drop source paths through a
manifest exclusion, or accept a Markdown-only edit as source re-review. Pages
build validates both baseurls through the Pages owner's pinned-theme checker.

Release publication uses protected release environments and job-scoped OIDC.
The official crates authentication action uses the reviewed v1.0.6 commit above.
Dependency-first waves include versioned dev dependencies and reauthenticate
before each wave (maximum eight reviewed waves; a deeper graph blocks until
the workflow is extended). Registry errors and existing immutable versions fail;
there is no blind rerun skip or token fallback. Partial reruns require artifact
identity review; automated identity-aware resume is not implemented. npm uses
Node 24/npm 11.16.0 trusted publishing with provenance. New npm trust settings
must explicitly permit direct `npm publish` (the current default grants staged
publication only), bind release.yml/release, and establish package ownership.

Current registry prerequisites are explicitly unverified in
scripts/ci/registry-prerequisites.json. All library manifests remain publish=false;
Rust runtime binaries, WASM/browser integration, isolated full Rust lab, native
startup pilots are not delivered by workflow files. The Pages owner has authored
the initial source/capability review manifest; concurrent Rust source changes
correctly make individual digests stale until an explicit renewed review.
The canonical source/test inventory must mark every responsibility
verified_runtime_parity with existing Rust paths and CI-visible parity_evidence,
or retired_with_review with an authored disposition and the same evidence.
Pending/unit-only rows and tracked Go source/manifests block final cutover.
Their absence fails validation/readiness instead of skipping. Native release
archives and retained defender GHCR SBOM/provenance/signing machinery
are wired but not publication-tested. Cross-OS Actions execution and live OIDC
must still be demonstrated after these prerequisites land. No publish occurred.

Official interface checks: [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/),
[Cargo package](https://doc.rust-lang.org/cargo/commands/cargo-package.html),
[reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows).

Design evidence is the inspected repository plus primary sources above. Code
and live account setup are pending. Implementation acceptance requires
cross-OS/pilot evidence, negative tests for the read-only freshness verifier,
locked source/site checks and a controlled OIDC release demonstration after
the product migration and account bootstrap prerequisites are met.


## Native probe subprocess-helper accounting

The audit key_custody harness has two explicitly ignored subprocess entry points,
not omitted product tests. Native Windows coverage must retain the exact Cargo
executable hash and distinguish required parent tests from these helpers. Accept
only the registered child_lock_probe and child_interrupted_writer in the audit
package/test target, with both matching parent tests passed, the lock helper's
actual nested successful summary and no unknown ignored/filtered tests. Report
ignored_helpers separately and preserve raw ignored_tests; never relabel them as
zero or count the nested child summary as an additional parent test. Any unknown
helper, missing parent/child proof, malformed extra summary or ordinary ignored
unit fails the gate. All other harnesses retain strict one-summary/zero-ignore
accounting. This does not skip tests or qualify full product runtime.
