# Rust delivery validation

**Diátaxis quadrant:** Reference. **Audience:** CI maintainers.

The reusable rust-validation.yml is the unprivileged PR/main/tag authority.
Workflow UI names use CI /, Docs /, Release / and Legacy / categories. The single
[workflow catalog](workflows.json) fixes technical file/job, concurrency and
artifact identities; Rust policy rejects unknown workflows or naming drift.
Filenames/job keys are stable API contracts, distinct from UI labels.
The frozen-go-ci.yml.txt and frozen-go-release.yml.txt files are immutable
historical snapshots, not executable workflows or regeneration consumers.
Workflow discovery reads registered YAML files in .github/workflows only.
RubyGems/PyPI entries are preparation-only planned publishers, not publishing
jobs. Package names and external publisher proof are pending; all readiness,
trust and direct-publish flags remain false. GitHub environment creation alone
does not configure a registry publisher or authorize an upload/token test.
Release reruns it on the exact tag source. Every required job must succeed;
skipped, cancelled, component-only and blocked outcomes cannot authorize shipping.
The frozen Go oracle and inert original workflows remain historical comparison
surfaces until reviewed final cutover. They are not Rust release authority.

New authored delivery implementation/tests are Rust1.99.1/edition2024 under
`scripts/ci/delivery`, registered in root's shared Cargo workspace/lock. The old
delivery.py, check_action_pins.py and test_delivery.py are retired. Upstream
action/Jekyll/theme tools and generated glue remain pinned external dependencies.

From the repository root:

```text
cargo test --locked -p humanymous-delivery
cargo clippy --locked -p humanymous-delivery --all-targets -- -D warnings
cargo run --locked -p humanymous-delivery -- policy
cargo run --locked -p humanymous-delivery -- action-pins
cargo run --locked -p humanymous-delivery -- audit-closure
cargo run --locked --manifest-path scripts/ci/docs-review/Cargo.toml -- .
cargo run --locked -p humanymous-delivery -- readiness
```

Readiness remains blocked by incomplete runtime/source parity, Go shipping
images, private/unpublishable manifests, native qualification receipts and
unverified external registry trust. Do not remove those gates to make CI green.

## Integration and evidence

`INTEGRATION.md` and the delivery SoT record exact accepted owner APIs. Native
execution belongs to the Rust native-runner owner; Docker topology and actual
container clients belong to test/lab/runner; Pages source/build checking belongs
to humanymous-pages. This CI tool aggregates those contracts without cloning
their implementation. Rust browser library/component success is not a real
human/AT/Pass recovery result.

`pilot native|browser|browser-product|docker` creates a fresh private Specfold bundle and runs
the owning tool with literal arguments. It records HEAD, working-byte SHA256,
canonical Git object identities, actual process exit, doc-review digest, cases
and images. A changed source, missing report/case, unknown failure, component-only
result or blocked owner exit rejects the full gate. Target/delivery-evidence
receives a sanitized summary; raw actor identity/paths/traces remain private.

Dedicated callers isolate one boundary without changing the full release gate:

| Workflow | Owning invocation and scope |
|---|---|
| `component-validation.yml` | `capture` runs five hosted transport cases; `header` checks cbindgen/C11/C++11 layouts. The dedicated native-artifact branch selects `archive`: build two release binaries, bind Cargo JSON/source/hash receipts, package and inspect actual archive contents without executing the binaries. |
| `wargame-validation.yml` | Scoped current-owner preflight followed by the existing generic Docker full gate. Preserve the strict original-byte 26-case receipt, independent raw/manifest hashes and unsupported counts; a receipt is not full-product success. |
| `go-validation.yml` | `go-product` consumes the exact eight-case bounded Go requirement. |
| `browser-product-validation.yml` | `pilot browser-product` retains the full requirement and its nonzero failure/blocked exit. |
| `mount-probe-validation.yml` | `mount-probe` collects four fixed custody mount observations. Complete refusals are diagnostic data, never custody or product qualification. |
| `image-validation.yml` | Reuses the full workflow's candidate build and typed image scan. Empty inventory, scanner failures and vulnerabilities remain failures. |
| `cef-native-component.yml` | The standalone Rust provisioner separates helper tests, native compilation, synthetic protocol, bootstrap rejection and desktop processes. GUI/AT readiness remains separate. |

These execution commands require their genuine supported hosted context and clean
exact workflow source. Ordinary validation branches do not write remote caches or
upload artifacts. Existing collector/CEF/cross-OS artifact opt-ins stay disabled
for this profile. Safe logs retain source/run accounting, counts and evidence
hashes; original private child transcripts are not recoverable merely from a
successful job or an emitted hash. CEF compiler projection excludes rendered
messages and unreviewed paths, and marks missing/unknown/truncated data explicitly.

`image-scan prepare|verify` binds the candidate image to source labels and typed
Trivy results. A successful empty scan is `not_scanned`; the actual runtime
binary must have an observed dependency inventory. The compiler-stage
cargo-auditable tool is pinned in `build/rust/runtime-build-tool.json`. Source
metadata and tool checksum checks alone do not qualify an installed tool,
runtime image, CVE scan or replacement for the legacy shipping images.

`observe-run ID EXACT_SHA PRIVATE_OUTPUT` records only that run/source, waits for
no unrelated result, and downloads its own terminal logs/artifacts. Later source
success never replaces an earlier failure. `register-evidence BUNDLE TITLE`
registers exact raw attachment hashes, not public reader prose.

## Documentation

The Rust docs-review command is read-only. docs/reviews.json schema1 covers the
exact crates/adapters/web/build/root Cargo source inventory with reviewed SHA256,
substantive notes, reader pages and fixed command/API/capability surfaces.
Changed source requires renewed authored review, not merely a Markdown edit.
Authored Rust must contain LF bytes. The Node surface now references
node/native-host-process, not a legacy JavaScript test script.

Pages uses `cargo run --locked -p humanymous-pages -- validate --source docs
--destination ABS --baseurl VALUE` for apex and /humanymous. Its checked artifact
is deployed only after the complete same-source Rust authority passes.
Each generated variant also passes the owning `package` and a second fresh
`archive-check`. Main transfers the verified `artifact.tar`, then validates the
downloaded archive before deployment readiness. Branch checks keep uploads off.

## Publishing boundary

Publication subcommands are implemented gates, not permission to publish.
Registry prerequisites remain explicitly unverified. External proof must bind
repository, release.yml, product environment (crates, npm, rubygems or pypi) and canonical evidence
digest. No long-lived registry token fallback is allowed. crates dependency
waves include versioned normal/build/dev paths and reauthenticate per wave.
Existing immutable versions are skipped only if registry name/version/checksum
matches this run's validated archive and source/lock/doc identities. A mismatch
or registry/auth/network failure stops; it never becomes an already-published
success. Normal Cargo archive verification and a repack digest check remain.

Native npm publication requires the agreed actual Node/Bun/authority receipt,
generated native provenance, advertised qualified tuples and validated archive
SHA. Publish consumes the same-run validated archive, not a fresh unvalidated
rebuild. Legacy JS conformance or synthetic-evidence ALLOW is insufficient.

Defender image publication retains max provenance/SPDX and keyless signing.
The Rust verifier checks each current-run digest, both Linux architectures,
exact Git source, nonempty per-platform SBOM and exact release-workflow OIDC
signature. It also verifies version/latest tags still resolve to that digest.
This code has fixture/negative coverage; live release verification and account
bootstrap are not claimed. Attacker images are never public release targets.

The prepared image matrix uses only `humanymous-runtime` and the existing Rust
runtime candidate Dockerfile on both Linux architectures. Candidate flags stay
false; readiness and a pre-login guard block publication until reviewed actual
source/platform/product/scan qualification exists. Old core/gate paths remain
forbidden shipping inputs and are retired by their owners, never silently
aliased to the runtime private-pipes component. This change grants no release.

CodeQL scans current Rust with the existing pinned action and `build-mode: none`.
The manual Go oracle uses only immutable baseline `981d25cf`, with automatic
uploads and remote cache writes disabled. Its historical results do not qualify
the current product or become a final Rust shipping dependency.
