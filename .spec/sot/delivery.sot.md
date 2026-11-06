---
id: sot:root/delivery
title: Rust delivery and Pages contracts
status: active
created: 2026-11-06T20:27:03Z
updated: 2026-11-06T20:40:50Z
verify: manual
rel:
  governs: [docs/**, .github/workflows/**]
  depends_on: [sot:root/rust-migration]
---

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

Design evidence is the inspected repository plus primary sources above. Code
and live account setup are pending. Implementation acceptance requires
cross-OS/pilot evidence, negative tests for the read-only freshness verifier,
locked source/site checks and a controlled OIDC release demonstration after
the product migration and account bootstrap prerequisites are met.
