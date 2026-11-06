# Rust delivery validation

`rust-validation.yml` is the single unprivileged validation authority. CI and
tag release call it without a ref input: each checkout uses the caller's exact
commit. Its final gate rejects failure, cancellation and skipped required jobs.
`frozen-go-oracle.yml` preserves the old lab for manual comparison only; it never
authorizes a Rust release. Frozen workflow originals are retained beside this file.

Local checks (Python 3.11+, Rust stable, Node 24):

```text
python -m unittest discover -s scripts/ci -p "test_*.py"
python scripts/ci/delivery.py policy
python scripts/ci/delivery.py readiness
cargo run --locked --manifest-path scripts/ci/docs-review/Cargo.toml -- .
```

Readiness currently fails intentionally. This is a shipping blocker, not a skip.
Never change a required job to optional merely to get a green migration branch.

## Implementation interfaces

Runtime owners must deliver the following executables. Validation invokes them
as argument arrays, never shell fragments. A runner must execute real assertions,
return nonzero on missing prerequisites/errors/skips, and emit the JSON report
specified below. File presence or an exit-zero wrapper does not prove parity.

* `scripts/migration/pilot-native.ps1`: `-Report <absolute path>` on all three OSes.
* `scripts/migration/pilot-browser.sh`: `--report <absolute path>`, Linux only.
* `scripts/migration/e2e-rust-docker.sh`: `--report <absolute path>`, Linux only.
  This must own isolated Docker topology, not delegate to a host loopback runner.

Reports have schema 1, `source_sha` equal to `git rev-parse HEAD`, `kind`
(`native`, `browser`, `docker`), `status: passed`, nonempty `cases`, and zero
`errors` and `skips`. `documentation_review_sha256` is the SHA-256 of the
current authored `docs/reviews.json`; each runner must actually execute that
surface's reviewed examples in its `documentation-examples` case. Each case has
unique `id` and `status: passed`. Required
case IDs are in `delivery.py`. Docker also declares `isolation: docker`,
`runtime: rust`, and nonempty `images` containing only sha256 image identities.
The report is evidence from that execution, not preauthored acceptance evidence.
CI removes its destination before launching, rejects absent reports, and checks
all fields. Honest report production remains part of the runtime runner review.

Node owner: package lives at `adapters/node`, has a lockfile, and exposes `test`
and `typecheck`, `test:pilot`, `test:bun`, `check:package` scripts. CI runs these,
npm ci, test, audit and `npm pack --json`;
package contents must omit tests/private evidence/attacker source. Release
version must exactly equal the Rust workspace version and strict tag.

Pages owner: `docs/Gemfile.lock`, `docs/Gemfile`, and
`docs/scripts/validate-pages.sh` implement locked install plus pinned-theme
source/built-site checks for both empty and `/humanymous` baseurl. The script
receives `--destination <absolute path> --baseurl <value>` and builds/checks
there, without generating authored prose. Delivery calls it twice, scans the
bundle for advisories, and uploads only the checked apex `_site` artifact.
Public deployment additionally requires complete Rust validation.

## Read-only documentation review interface

`docs/reviews.json` schema 1 contains `reviews`, one record for **every** file
under `crates/`, `adapters/`, `web/`, `build/`, and root Cargo.toml/Cargo.lock.
Excluded directories are only `.git`, `node_modules`, `target`, `.bundle` and
`vendor`. Each record has `source`, lowercase SHA-256 `sha256`, nonempty
`reviewer`, meaningful `notes`, and nonempty `pages` naming existing authored
Markdown files under docs. Page paths cannot escape the repository. Reviews
must explain what the governed source means for the reader, including unchanged
prose decisions. Source coverage cannot be reduced by a list in the review file.
The manifest also requires `surfaces` records for every fixed command/API/config/
capability ID in `docs-review/src/main.rs::SURFACES`. Each has `id`, `sources`,
`pages`, `reviewer`, `notes`, and `checks`; sources and pages must join to reviewed
records, and checks must include that surface's required same-commit pilot case.
This prevents a set of source digests without capability/example coverage from
passing. New public capabilities require extending this reviewed fixed inventory.
The Rust verifier checks exact inventory, digests, page existence and front
matter. It never generates or repairs evidence/prose. Executable examples are
separately required by native/browser/Docker/Node/Pages jobs at the same SHA.

Registry ownership/bootstrap and protected environment/trust bindings are
external prerequisites. `registry-prerequisites.json` deliberately records them
unverified. A maintainer must review each binding and evidence before changing
it. There are no secret-token fallbacks. crates publication is dependency-first,
reauthenticates per crate, and rejects existing versions rather than guessing
whether an interrupted publish produced identical archives. Partial publication
requires explicit artifact identity review before any resume procedure exists.

Full migration readiness also reads the canonical Go source inventory at
`.spec/evidence/20261106194112-go-migration-source-inventory/go-inventory.json`.
Every row needs `parity_status: verified_runtime_parity`, real `rust_paths`, and
nonempty `parity_evidence` naming CI-visible `.spec/evidence/` files. An actually
retired responsibility uses `retired_with_review` plus substantive `disposition`
and parity evidence. Unit-only/pending rows never authorize shipping. The final
tracked tree must contain no Go source/manifests, including migration-only Go
generators; frozen oracle retention during development is deliberately distinct
from readiness for final cutover. The inventory is read-only from this lane.
