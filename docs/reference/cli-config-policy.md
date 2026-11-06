---
layout: doc
title: Command and configuration contract
permalink: /reference/cli-config-policy.html
reviewed: 2026-11-07
---
# Command and configuration contract

**Diátaxis quadrant:** Reference. **Audience:** operators and adapter authors.

The Rust command line and complete configuration schema are not release-qualified.
This page deliberately supplies no executable installation example.

The intended CLI reports actual topology, capabilities, route conflicts and
management exposure before enabling protection. Configuration must distinguish
required observations from optional telemetry, bind decisions to a policy
revision and reject unsupported settings. Missing required evidence fails
closed; it does not become a fabricated allow decision.

Published command help, schema defaults and examples must be tested against the
same release artifact. Existing Go flags are not a Rust compatibility promise.
See [installation requirements](install-requirements.md).
