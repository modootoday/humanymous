---
layout: doc
title: Prepare a monitor-mode evaluation
permalink: /tutorials/quickstart-monitor-mode.html
reviewed: 2026-11-07
---
# Prepare a monitor-mode evaluation

**Diátaxis quadrant:** Tutorial. **Audience:** first-time evaluators.

This is a planning exercise. A runnable Rust monitor-mode tutorial awaits a
qualified binary, configuration schema and end-to-end pilot. No installation
command is implied by this page.

Create a diagram of a synthetic client, enforcement edge and disposable origin.
Mark the TLS endpoint and management boundary. Next, write three expected cases:
a legitimate request, a request with missing required evidence and an audit sink
outage. For each, record the expected observation, decision and origin effect.

Finish by describing how the lab is isolated and how you will recover it. This
produces a reviewable evaluation plan, not a runtime result. Continue with the
[evaluator guide](../start-here/evaluator.md).
