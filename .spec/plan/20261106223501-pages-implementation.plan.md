---
id: plan:root/20261106223501
title: Authored Pages execution and remaining gates
status: active
created: 2026-11-06T22:35:01Z
updated: 2026-11-06T22:35:01Z
rel:
  depends_on: [sot:root/pages-implementation]
---

# Authored Pages execution and remaining gates

## Goal

Implement [[sot:root/pages-implementation]] within the docs lane, with reviewed
source, reproducible local builds and an attributed checkpoint. Runtime release
and public publication remain outside this lane.

## Steps

- [x] Claim docs lane and read migration, adapter, delivery, standards and research canon.
- [x] Verify theme v1.5.0 SHA and supported token bridge/checker from source.
- [x] Rewrite legacy public pages as authored English with honest migration limits.
- [x] Preserve seven standards research pages, native navigation and stable URLs.
- [x] Add locked maintained Jekyll/remote-theme dependencies and remove vulnerable RubyZip selection.
- [x] Implement read-only source and built-site validation interface expected by CI.
- [x] Run final apex and /humanymous builds plus exact-theme strict checks.
- [x] Record contrast, keyboard/light/dark/mobile browser observations.
- [x] Complete current 201-source review inventory and 12 required capability/check mappings.
- [x] Remove obsolete tracked OG/LLM/screenshot/image assets through bounded git-mutex transaction.
- [x] Run seven source/output negative cases and public Specfold check; prepare attributed checkpoint.
- [ ] Release docs/git-ops after the checkpoint transaction.

## Progress

Initial apex render and theme strict check passed for 77 authored pages. The
added built-site checker found a filename normalization bug for a name ending
in '-index.html'; fixed it to match the exact basename index.html. Final reruns
passed for both baseurls. Source and output checks pass for 77 pages. Ruby advisory DB commit
516ea4757d5eec778c49af8371c23309c4b70f42 (2026-11-04) identified CVE-2026-85396
on RubyZip 2.4.1; updated remote-theme 0.6.3/RubyZip 3.7.0 and audit passed.

The broad recursive legacy-asset removal was twice rejected by automatic approval
review, including after explicit user authorization. No deletion happened from
those commands. Text generators/layout/CSS were removed through explicit patches;
the subsequent bounded git rm of tracked retired asset paths succeeded under
the shared git mutex. It removed only the owned, reviewable tracked paths.

Chrome 155.0.8059.86: 72 light/dark and 1440/390px cases over both baseurls,
zero axe A/AA violations and zero document overflow after the preparation-page
wording fix. Eight skip-link keyboard checks passed. Selected text token ratios
range from 5.587 to 16.223. Mobile screenshots were visually inspected. Private
theme/tool npm audits report zero known vulnerabilities. Seven validator boundary
tests pass. The full screen-reader audit and remote Actions/deployment remain
unexecuted; no runtime or standards-conformance acceptance follows from these.

Current scoped source reviews pass for 201 source files and 12 required surfaces.
Mappings retain unqualified runtime status; check IDs name required executions,
not completed pilots. Reviews were explicitly renewed for the newly expanded
record/checkpoint corpus and JSON escaping changes after parallel edits made
the freshness gate fail stale. Ongoing source changes must be re-reviewed.

**Cross-OS freshness blocker discovered before checkpoint:** core.autocrlf=true
leaves 53 existing build/web assets as CRLF in this Windows checkout while Git
stores LF. The CI-owned Rust verifier hashes raw working-tree bytes, so the
current locally passing review digests differ on a fresh Linux checkout for
those sources. Evidence `freshness-line-endings.json` records both digests and
exact paths. Main/CI owner must choose a canonical policy: preferably enforce
LF text checkout in root attributes and review that canonical inventory, or
explicitly normalize recognized UTF-8 text in the verifier while hashing binary
files unchanged, with CRLF/LF/binary negative tests. Do not just refresh hashes
per runner or claim cross-OS freshness acceptance. No out-of-lane source or
verifier was changed to conceal the issue.

Public `spec check --no-local` passes. Full `spec check` still reports eight
unregistered files in the separate Node adapter private evidence bundle, outside
this lane. Own evidence attachments are registered; binary ZIP/PNG secret-scan
warnings are expected and their underlying public text was separately checked.

Remaining integration gates: same-commit remote Actions run; source review after
further runtime changes; actual Rust/native/browser/Docker/example acceptance;
final artifact dependency scans including OS packages of any retained builder;
user-authorized publication after the complete migration. No publish, push,
tag, main merge or external standards communication occurred here.

Docker Linux 29.9.2 was available. First dependency resolution used Ruby 3.3.10;
final validation uses Ruby 3.3.12-bookworm pinned digest
sha256:059dd8c61de027b41a1a4f808698f7c59895d4e861cb78a84582c7f7f29a8663.

## Verify

From docs with a locked installed bundle:

```sh
python3 scripts/check-site.py source
bundle exec bundle-audit check --update
bash scripts/validate-pages.sh --destination /tmp/pages-apex --baseurl ''
bash scripts/validate-pages.sh --destination /tmp/pages-subpath --baseurl /humanymous
```

From repository root:

```sh
cargo run --locked --manifest-path scripts/ci/docs-review/Cargo.toml -- .
spec check
```

Actual logs and browser artifacts remain private. No remote Pages build/deploy,
external standards submission or Rust runtime acceptance is asserted.
