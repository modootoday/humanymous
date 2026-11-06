---
layout: doc
title: Prepare a detector integration test
permalink: /how-to/run-detection-engine.html
reviewed: 2026-11-07
---
# Prepare a detector integration test

**Diátaxis quadrant:** How-to. **Audience:** implementation evaluators.

A qualified standalone Rust engine command is not documented yet. Define exact
input observations and expected outcomes from the frozen comparison corpus.
Exercise ordering, boundary values, missing required inputs and invalid arithmetic.

Record source/artifact identity and compare complete traces where the contract
requires them. A detector result does not prove edge enforcement; follow it with
the owned Docker integration suite once wiring is ready.
See [computation boundaries](../explanation/detection-engine-internals.md).
