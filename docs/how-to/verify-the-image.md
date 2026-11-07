---
layout: doc
title: Review a candidate runtime image
permalink: /how-to/verify-the-image.html
reviewed: 2026-11-07
---
# Review a candidate runtime image

**Diátaxis quadrant:** How-to. **Audience:** artifact reviewers.

No final Rust runtime image is qualified here. For a future candidate, obtain
its immutable digest, source revision, provenance and same-commit validation
report through the release's documented verification mechanism.

Check that the image contains the intended Rust runtime and excludes attack
tools, private evidence and credentials. Confirm startup, non-root permissions,
management isolation and shutdown in the owned lab. Signature verification
establishes artifact identity under a trust policy, not functional correctness.
See [release status](../reference/production-vs-reference.md).

The experimental nginx component image is explicitly a laboratory artifact. Its
final stage includes the nginx engine/module, the pilot runner and an authority
executable, and starts the pilot. Do not mistake it for a minimal production
runtime image. The Compose declaration requests non-root, network isolation,
read-only root, dropped capabilities, no new privileges and bounded temporary
storage/processes/memory. Inspect the actual container and test outcomes before
claiming those controls were effective; configuration text alone is insufficient.
See the [nginx component boundary](../reference/supported-topologies.md#experimental-native-nginx-component)
for historical versus current-source qualification and synthetic ALLOW limits.

Identify the fixture before interpreting a container result:

| Fixture declaration | Review boundary |
| --- | --- |
| Authority lab | Rust authority with a historical Node harness/adapter; not the new native Node-API package or full browser flow |
| Custody check | Compiled audit/authority unit and subprocess tests; the default entrypoint runs only the audit unit binary, so inventory every actually invoked test |
| Collector browser | Firefox WASM component tests with historical read-only differential oracles; Firefox is not the product's native UI engine |
| Product browser | Runtime-pilot browser driver plus a selected collector WASM test artifact. A compiled test module or headless Firefox launch does not prove all tests, human controls or native UI behavior ran; this is separate from Pages keyboard validation |
| Apache native candidate | Pinned upstream httpd/APR build with selected generated bindings, normal and test-probe modules, and a separate pilot stage. It is a laboratory candidate, not an installed or qualified server tuple |
| Runtime pilot | Real runtime/CLI process harness; separate from the full isolated product deployment and human/assistive controls |
| Historical lab clients | Python fixture controls; not a newly implemented Rust client or authenticated browser collector |

A pinned image or checksum identifies selected inputs under a trust policy;
it does not prove all upstream packages are immutable or current-source execution
passed. Source context exclusions, final image contents and runtime isolation
are different checks. Keep test-only foreign-language oracles and old fixture
results separate from current Rust implementation claims. Neither a built test
binary nor an image entrypoint establishes the full suite actually ran.

The Apache candidate recipe disables upstream SSL and HTTP/2 and selects event
as its default MPM while building shared MPM choices. That does not qualify every
MPM, original-client TLS/H2 capture, pool cleanup or graceful reload. Record the
actual configured headers, loaded compiler/libclang and built module/image
identities; archive/header pins alone do not establish the configured ABI.
Its non-root image user and private configuration expectations do not prove
effective network or origin isolation.

The product-browser recipe pins a geckodriver archive digest, but installs other
browser/system packages through its declared package manager. Capture the actual
browser/tool versions and immutable built image rather than treating the base
image pin as a complete dependency closure. Its build context includes Pages
tooling for workspace resolution, not the rendered documentation site. Neither
that context nor a product-browser result supplies a current Pages build or
keyboard/assistive-technology report.
