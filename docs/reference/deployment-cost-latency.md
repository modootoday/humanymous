---
layout: doc
title: Deployment cost and latency evidence
permalink: /reference/deployment-cost-latency.html
reviewed: 2026-11-07
---
# Deployment cost and latency evidence

**Diátaxis quadrant:** Reference. **Audience:** capacity planners.

No measured Rust production capacity, cost or latency budget is published here.
Historical measurements cannot be transferred across implementations or topologies.

A useful pilot reports hardware, resource limits, artifact/configuration,
request and body distributions, concurrency, cache state and audit storage. Show
latency distributions, throughput, CPU, memory, disk growth and failure rates.
Include overload, backpressure and recovery rather than only steady-state success.

Report browser challenge friction separately from server processing latency.
Bound storage and queues explicitly; an unbounded buffer is not a capacity plan.
See [evaluation reporting](../explanation/transparency-report.md).
