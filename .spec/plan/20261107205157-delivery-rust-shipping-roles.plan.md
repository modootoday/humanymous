---
id: plan:root/20261107205157
title: Rust shipping role replacement and legacy image retirement dependencies
status: active
created: 2026-11-07T20:51:57Z
updated: 2026-11-07T20:51:57Z
rel:
  depends_on: [sot:root/delivery, sot:root/structural-development, sot:root/runtime-application, plan:root/20261107061820]
---

# Brief

Replace or explicitly retire the two legacy Go image roles without relabeling a
different Rust component as an equivalent product. Root authorized this read-only
map and owns common graph changes and retirement sequencing. Required image jobs,
scoring freeze, private authority, accessibility and the original Git-only981d
oracle remain intact. No Dockerfile, workflow, public deployment or product code
is changed by this plan. This is a dependency proposal, not implementation approval.

# Source and observed outcomes

The path/hash inventory is attached under [[evidence:root/20261107052001]] as
shipping-role-source-map.json. It binds inspected current source bytes, not an
immutable whole-workspace build. Old hosted run42407744891 at9586 separately built
and scanned the runtime candidate: one observed Rust binary inventory,91 packages
and zero reported vulnerabilities. Inventory completeness and shipping remained
false. Both legacy image jobs stopped at the global image-sources guard naming
build/core.Dockerfile; neither performed its Docker build or vulnerability scan.
That is not a measured Gate image failure and is not permission to weaken the guard.

# Role vocabulary and exact dependency map

| Role ID | Existing shipped boundary | Available Rust candidate | Dependency before replacement |
| --- | --- | --- | --- |
| image.core.demo | build/core.Dockerfile builds cmd/server and the Go WASM/browser loader; serves web assets, terminates public TLS on443 with ACME cache; lab overrides use8443 | humanymous-core is a pure SDK library; humanymous-runtime is a private-config loopback application process | Runtime and Browser owners must accept a standalone detection/demo composition, its browser assets and original TLS/H2 provenance, or Root must explicitly retire this public role and approve the replacement required contract |
| image.gate.edge | build/gate.Dockerfile builds cmd/gate; edge8444 plus separate admin8445, upstream origin, routes file, audit/keystore data and ACME cache; release Compose publishes443 and loopback admin, probes and drain | Runtime has application/session, optional TLS/management and durable audit primitives; CLI has runtime-host/native service lifecycle | Runtime and CLI owners must supply an accepted container composition for the public ingress/origin topology, authenticated administration, persistence, certificate provisioning, health/readiness and signal-driven shutdown |
| image.runtime.private | build/rust/runtime-candidate.Dockerfile builds humanymous-runtime, scratch/nonroot65532, inherited private configuration and readiness pipes | This is the actual source9586 candidate that was built/scanned | Keep its private role identity; add actual current-source container lifecycle, retained inventory/scan and platform evidence before any shipping claim; it does not satisfy either row above by renaming |

The role table is the minimum concrete admission surface exposed by the current
Dockerfiles and shipped Compose consumers. It is not a completed inventory of all
legacy flags, optional integrations, admin actions or deployment environments.
Owning review must close that inventory before claiming whole-role equivalence.

# Why an immediate Dockerfile substitution is refused

1. Runtime main rejects any extra argv and terminal stdin/stdout, then reads its
   private configuration pipe. Existing Compose passes Go command-line options.
2. Runtime Configuration.validate requires both listen and origin addresses to be
   loopback. Existing roles expose public ingress and a separate origin container.
   Mapping ports or changing an image name does not alter this validated topology.
3. The standalone readiness profile is explicitly experimental-loopback-http1;
   original_h2, browser_collector and product_qualified are false. Optional TLS-H1
   primitives do not establish original H2, collector or standalone-demo parity.
4. Runtime main has no platform shutdown adapter. CLI's runtime-host owns the real
   shutdown sender and native admission; it needs an accepted container provider,
   not an unrelated executable copied into /app/server or /app/gate.
5. The Authority process requires private IPC configuration and emits a restricted
   adapter-authority readiness contract. It is not a public reverse proxy.

# Ordered owner handoff and retirement map

1. Root and Runtime close a source-bound role inventory for Core and Gate. Classify
   every advertised behavior as implemented and measured, an explicit capability
   refusal, or an explicitly accepted retirement. Preserve frozen scoring semantics.
2. Runtime owns the actual ingress/origin/collector/capture/authority composition.
   Public ingress must preserve owned TLS/H2 observations or explicitly refuse
   unavailable planes. A normalizing external proxy is not original-client proof.
3. CLI owns a container lifecycle adapter using the accepted private configuration,
   sealed custody, readiness and shutdown APIs. It must retain real cancellation,
   drain and final flush outcomes, secret handling and external-parent ceilings.
   Do not add an unreviewed secret environment/argv fallback or grant broad binds.
4. Browser owns the demo/static collector artifact and protocol if Core remains a
   public demo. Collector/component green does not qualify a working browser flow.
5. Delivery then wires the approved executable/configuration contract into pinned
   Rust-only image recipes, source and license inventories and required CI roles.
   Preserve both required role checks until Root approves a concrete replacement
   graph; no optional jobs, unconditional pass, missing-file shortcut or aliases.
6. After that acceptance, Root may remove obsolete Go recipes, loader/build paths
   and deployment consumers together or replace them with the accepted Rust role.
   Do not retain active Go compilation as a compatibility bridge. The981d oracle
   remains immutable Git history, never copied into the new product image.
7. Root sequences exact-source hosted build/scan and Docker-only product validation.
   Publication, deployment, tags and account/OIDC exchanges are outside this work.

# Verification required before a shipping claim

Pure units and strict compile/lint/format prove selected code only. Each retained
role additionally needs an actual pinned Docker build and image inspection,
nonroot/capability/read-only-volume checks, source/binary/notice binding, a retained
nonempty expected-versus-observed dependency inventory and all-severity CVE result.
Record OS/arch, tools, image ID, database/report hashes and original raw availability.
Cross-compilation does not qualify execution on a second architecture.

Actual owned Docker topology must exercise the accepted public entrypoint, real
TLS/H2 and browser prerequisites, origin effects, authenticated admin denial and
allowed actions, durable audit recovery, readiness during drain, signal shutdown,
failed cleanup and storage failure. Missing required signals remain fail-closed;
Pass/CHALLENGE cannot require motor richness or speed. Neither the private-runtime
candidate nor the legacy Git oracle substitutes for these role-specific results.

# Critique reflected

This is one author's review using the blue/red/SRE/DPO/evaluator lenses, not an
independent multi-agent panel. Verdict: needs-work; no source replacement authorized.

- P0: renamed private-runtime binary would silently change public role semantics.
  The role map and owner acceptance above are prerequisites to recipe changes.
- P0: widening loopback or substituting forwarded headers would bypass a validated
  trust boundary. Require an owning topology/capture decision and actual tests.
- P1: SDK, component or clean observed-inventory results could be promoted to a
  product image. Require separate retained role/build/runtime/CVE evidence.
- P1: removing Go inputs alone could erase required coverage. Retirement must update
  the accepted role contract, consumers and positive required gate together.
- P1: a container wrapper could lose private custody, signal drain or audit flush.
  CLI/Runtime must own and measure the lifecycle adapter before packaging it.

# Next action

Root assigns the missing Runtime/CLI/Browser composition contracts before Delivery
changes either legacy Docker role. The existing image-sources refusal remains
required. Platform consolidation and the sole hosted watch continue independently;
this plan consumes no compiler, container, deployment or hosted-dispatch lease.
