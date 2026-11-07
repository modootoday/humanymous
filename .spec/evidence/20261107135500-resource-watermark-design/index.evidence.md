---
id: evidence:root/20261107135500
title: Resource watermark source investigation
status: active
created: 2026-11-07T13:55:00Z
updated: 2026-11-07T14:02:49Z
rel:
  depends_on: [sot:root/resource-watermark, plan:root/20261107135500]
files:
  - {path: source-inventory.json, sha256: fb4f873c21b92290a281dde7249f6a0950c2bfd04f9cd87b824ad3efb1197084, bytes: 8252, store: repo}
  - {path: initial-source.json, sha256: b1fa15e8bb933a72e48891cc7a92e2763d8dd62e70de03137a062c902995d114, bytes: 3260, store: repo}
  - {path: pinned-api-review.json, sha256: 5785b780477877823a592537c46c5b0283213af64ee49349b4e9f3d4233263b4, bytes: 1840, store: repo}
  - {path: codec-dependencies.md, sha256: 1ac7ab7f1838a6edf878026ef2c3bbed92a6662bf27086a9e9e265f6edb52a73, bytes: 6261, store: repo}
  - {path: integration-checkpoint.json, sha256: a99bcf736b374cfdc80c4d3d29a226367ccf9406c3359a7cfa613d7d141a873f, bytes: 948, store: repo}
---

# Source investigation receipt

## Atomic workspace source integration

Root committed all ten frozen package files, workspace member/lock edges and its
migration decision together at9b064dbd8942d4fad7f1623da97b6b64c0177a7f. The
owner independently inspected the exact13-file change, matching package hashes,
empty package diff against that commit, and LF in both index and worktree.
`integration-checkpoint.json` records this source-bound checkpoint. Root's
migration SoT records offline and locked workspace tree resolution; this owner
has not run Cargo and does not relabel graph resolution as compilation/test.
Sixteen authored tests remain NOT_RUN. Codec, actual resource, HTTP/operator
trace and Docker product gates remain incomplete. Full goal is not achieved.

## Accepted first-slice continuation

Root accepted the new source-reconstructed canon and package location, W01–W09
corrections, explicit nonzero TTL and component admission limits, and granted
source-only payload/carrier plus bounded volatile ledger/reservation/cancellation.
Original SoT-08 remains NOT_FOUND; that explicit disposition resolves design
authority without claiming the original was read or final parity achieved.
`initial-source.json` binds the ten authored package files and four existing
pilot candidates. The manifest is frozen for root-owned workspace/lock integration.
Six payload and ten ledger tests are authored, zero executed. Direct rustfmt
source formatting exited0; no Cargo/compiler/test/pilot/Docker command was run.
The RFC4231 case-one vector was checked against the primary RFC Editor source:
[RFC4231 section4.2](https://www.rfc-editor.org/rfc/rfc4231#section-4.2).
This checks the cited literal, not the unexecuted Rust test or cryptography.

Read-only fixture review found no tracked .wasm file. PNG sample IHDR advertises
96x96 RGB8; decoder validity remains untested. Full legacy HTML depends on the
Go WASM loader and is not an isolated semantics proof. The plan now records the
eight original named tests as explicit preservation obligations and distinguishes
existing candidates from fixtures still requiring construction/approval.

## Initial read-only design checkpoint

Read-only source investigation plus draft spec preparation. No build, test,
Go execution, Docker, browser, actual-resource pilot or deployment occurred.
Source existence is not parity. See source-inventory.json for fourteen exact
baseline/current rows and read consumer hashes. Original baseline remains intact.

Investigation began at HEAD6a3678eaa45c1f49f362ce7f92ea79c98f0b65a8 in a shared
dirty checkout; the repository may advance independently. No clean-tree claim.
The misc lane was free when listed and claimed by rust-resource-watermark for
only the named watermark SoT/plan/evidence paths. No implementation is authorized.

Read in full: all fourteen internal/watermark/*.go, cmd/server/resource_handlers.go,
cmd/server/helpers.go, test/redteam/watermark_strip.mjs. The routing document's
fourteen rows point to /files/298 through /files/311 in the immutable314-row
baseline. All source hashes are compared explicitly in the JSON attachment.

Searches: rg filename/content searches under crates found no watermark module or
consumer. Historical consumers were found in cmd/server/server.go and resource
handlers; these remain read-only. SoT-08 was not found by current tree filename,
baseline Git tree, all-history sots/08 path log or local spec snapshot searches.
An absent original is an unresolved provenance gap, not a reconstructed citation.

Observed corrections needing root acceptance are listed in W02–W09. The original
test names cover eight tests; no original or Rust test has been executed here.
The invalid minimalPNG and magic-only WASM fixture cannot establish actual format
semantics. The PNG strip oracle does chunk filtering; pixel re-encode proof comes
from a different legacy unit responsibility and needs a real Rust pilot.

Document validation: full `spec check` exited1 with72384 errors and14 warnings,
including existing local owner/type layout and unregistered evidence attachments.
Scoped `spec check` over the three new watermark document paths with `--no-local`
exited0, zero errors and zero warnings. An initial scoped warning for unknown
rel.supports was corrected to rel.depends_on and the scoped check repeated.
The inventory attachment is registered with digest and byte count in frontmatter.
Fourteen source hash comparisons and registry digest comparison match; this is
source preservation evidence only. No executable verification gate was attempted.
