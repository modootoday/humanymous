# Rust delivery validation

The reusable rust-validation.yml is the unprivileged PR/main/tag authority.
Workflow UI names use CI /, Docs /, Release / and Legacy / categories. The single
[workflow catalog](workflows.json) fixes technical file/job, concurrency and
artifact identities; Rust policy rejects unknown workflows or naming drift.
Filenames/job keys are stable API contracts, distinct from UI labels.
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

`pilot native|browser|docker` creates a fresh private Specfold bundle and runs
the owning tool with literal arguments. It records HEAD, working-byte SHA256,
canonical Git object identities, actual process exit, doc-review digest, cases
and images. A changed source, missing report/case, unknown failure, component-only
result or blocked owner exit rejects the full gate. Target/delivery-evidence
receives a sanitized summary; raw actor identity/paths/traces remain private.

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

## Publishing boundary

Publication subcommands are implemented gates, not permission to publish.
Registry prerequisites remain explicitly unverified. External proof must bind
repository, release.yml, product environment (crates or npm) and canonical evidence
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
