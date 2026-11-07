---
layout: doc
title: Prepare a detector integration test
permalink: /how-to/run-detection-engine.html
reviewed: 2026-11-07
---
# Prepare a detector integration test

**Diátaxis quadrant:** How-to. **Audience:** implementation evaluators.

Use a reviewed checkout and Rust 1.99.1. The following runs source-bound
component tests; it is not a full deployment test:

```console
cargo test --locked -p humanymous-scoring -p humanymous-signals
cargo test --locked -p humanymous-runtime
```

Define exact input observations and expected outcomes from the frozen comparison
corpus. Exercise ordering, boundary values, missing required inputs and invalid
arithmetic. The runtime includes a real child-process pilot with owned synthetic
origin/audit effects; those client reports are assertions, not real human controls.

Record source/artifact identity and compare complete traces where the contract
requires them. A detector result does not prove edge enforcement; follow it with
the owned Docker integration suite once wiring is ready.
See [computation boundaries](../explanation/detection-engine-internals.md).
