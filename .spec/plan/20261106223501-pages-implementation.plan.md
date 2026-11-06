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
- [x] Create attributed checkpoint 11f2f3c; release git-ops after the transaction.
- [x] Prepare docs integration handover; release lane via session board after cleanup commit.

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

**LF policy selected by user:** added root `.gitattributes` with
`* text=auto eol=lf` and explicit binary rules. Of 53 files containing CRLF bytes,
52 are UTF-8 text; the PNG header's CRLF is binary and must not be normalized.
Renewed the ledger for canonical LF text, retaining exact binary hashes. A
canonical export of all 201 governed files passes the unchanged Rust verifier.
Actual Git clean-filter tests cover LF/CRLF text and unchanged binary bytes.
Evidence: `lf-policy.json`, `git-lf-policy-check.json`, `canonical-review.log`.
No shared runtime working file was rewritten while its owning session edits it.
Main must safely refresh existing CRLF working text or use a fresh checkout;
otherwise its raw-byte local review will remain stale. New Windows/Unix
checkouts enforce LF. Real cross-OS Actions execution remains pending.

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

## Handover for main integration

Mission: complete the authored Pages migration without overstating the unfinished
Rust product. Docs lane can be released; no local implementation work is running.

Done: checkpoint 11f2f3c replaces legacy pages/layout/generators/assets, pins
theme v1.5.0 SHA, provides locked Jekyll builds and read-only checks. Both actual
baseurl builds pass 77-page theme/link/canonical checks. Chrome's 72 combinations
and eight keyboard checks pass; seven validator boundary tests and Ruby/theme/tool
dependency audits pass. Raw evidence is stored privately under
`.spec.local/root/evidence/20261106223502-pages-build/`, and the
inherited standards pilot now records actual execution. A follow-up removes four
accidentally staged Python cache files and adds a local script ignore rule.

In progress: no docs process remains. Runtime work continues in main/other lanes.
The current 201-source ledger is valid on the reviewed Windows tree only.

Next actions: main safely applies the user-selected LF checkout policy to its
existing shared worktree, renews source reviews after runtime changes and runs
same-commit native/browser/Docker and
remote Actions gates. Resolve the separate Node private-attachment metadata
errors before claiming full spec check. Scan retained builder OS packages before
calling the final dependency audit complete. Publish only after authorized full
migration acceptance; do not deploy this local preview as a qualified runtime.

Verify commands are above. Read AGENTS.md, `.agents/lessons/HARD-WON.md`, this plan,
[[sot:root/pages-implementation]], docs/reviews.json and the CI-owned
scripts/ci/README.md. Persistent details are in these files, not provider-only
memory; this residual chat is Codex-owned. Detection scoring/predicates/bands did
not change. Main merge, tags, push, publication, third-party probing and external
standards communications are outside this lane.

## User-facing local preview

The verified apex artifact is open as a retained deliverable tab in the Codex
in-app browser at http://127.0.0.1:8765/. The loopback-only Python static server
serves only the public built tree at
`C:/Users/smile/.codex/tmp/humanymous-pages-20261106223502/polish-apex`.
It is not a public deployment. This user-requested preview should remain running
while the user reviews it; server exec session is 58554.

## Public terminology audit requested by user

Scope: current in-progress docs only; deployed humanymous.net is explicitly
excluded. Completed read-only review of all 77 reader sources and 193 built
files. Inventory and source hashes are private under
`.spec.local/root/evidence/20261106222723-pages-public-terminology-audit/`.
No public content or runtime was modified by this audit.

Findings and proposed follow-up:

- Public requirement labels P-01..P-10 are legitimate local draft identifiers,
  not priority levels or assigned standards IDs. There is no standalone P0/P-0.
  However profile.md:41 and conformance.md:36 use an unexplained prefix and
  code-only references. Add descriptive requirement names, define the label
  scope, stable anchors and requirement-specific conformance links. Ten
  definitions and nine literal displayed references/range endpoints are present.
- security-privacy.md:31 and :57 use uppercase MUST NOT in an initial self-review
  without its own BCP 14 scope or explicit normative cross-reference. Either
  make the review informative in ordinary language and point to the requirements,
  or declare the intended project-normative scope. This is clarity, not evidence
  of an external adoption claim.
- Expand less familiar ABI (supported-topologies.md:19), SSRF
  (security-privacy.md:28), RFCXML/IPR/TAG (preparation.md:39/:63/:64) on first
  use or link clear definitions. These are normal technical terms, not internal
  project codes. Existing protocol RFC numbers, BCP 14, HTTP versions, Draft v0,
  Internet-Draft -00 and synthetic wire fields should remain where needed.
- Production output includes 43 CSS source maps with temporary upstream theme
  build paths. Consider sass sourcemap: never after checking converter support.
  This is optional public-surface cleanup; no private spec ID/key hit was found.

No internal SoT/private paths, tier/layer/hard-rule/gate codes or project
phase/ADR/sprint/TODO markers were found in rendered reader content. All
decodable output assets also passed the bounded private-ID/key-pattern scan.
Primary references: https://www.rfc-editor.org/rfc/rfc8174.html and
https://www.w3.org/TR/qaframe-spec/. No new runtime conformance, complete secret
audit, or publication claim follows. Corrections await implementation scope;
the user asked for investigation, so authored pages were preserved.

## Public terminology corrections authorized by user

Implemented after the user requested improvements:

- Preserved public P-01..P-10 references, defined P as profile scope and explicitly
  distinguished identifiers from priorities/standards assignments. Each now has
  a descriptive heading. Expanded the validation matrix to ten named, individually
  linked requirements instead of opaque ranges; real built anchor checks pass.
- Marked the security/privacy page as informative. Reworded the two uppercase
  MUST NOT statements as review checks and linked the governing public requirement
  or accessibility contract. No security/detection/accessibility policy changed.
- Expanded ABI and SSRF at first use; clarified RFCXML and editorial/nits checks,
  IPR and TAG references. Added all five public glossary definitions.
- Disabled CSS source maps using the supported Sass setting. Added a built-site
  rejection and negative test for debug maps. Actual fresh builds contain zero
  maps, down from 43. The validator now has eight passing boundary tests.

Locked actual apex and /humanymous builds pass exact-theme --strict plus all
77-page local target/anchor/canonical checks. Chrome 155.0.8059.86 passed 88
page/baseurl/light-dark/desktop-mobile combinations with zero axe A/AA
violations and zero page-width overflow. Eight keyboard skip checks passed.
The local loopback preview was switched to the new artifact, refreshed in the
in-app browser and retained as a deliverable; expanded RFCXML/IPR/TAG text was
visibly confirmed. Built output is 150 files with zero maps or sourceMappingURL
directives; all ten requirement anchors and matrix links exist. Private evidence bundle:
`.spec.local/root/evidence/20261106223158-pages-public-terminology-improvements/`.
No currently deployed-site audit or publication is part of this scope.

Global `spec check --no-local` is currently blocked by the separate wargame
plan's link to private evidence (and its missing verification warning), outside
the docs lane. This task's changed documents do not introduce those findings.
No runtime/source-review hashes were refreshed to conceal parallel code changes.
