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
