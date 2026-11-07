---
id: plan:root/20261107151000
title: Bottom-up implementation pyramid and complete chat ownership
status: active
created: 2026-11-07T15:10:00Z
updated: 2026-11-07T15:10:00Z
rel:
  depends_on: [sot:root/program-roadmap, sot:root/rust-migration, plan:root/20261107150000]
---

# Accepted bottom-up delivery direction

The user requested implementation from lower modules upward, verified through
units and integration like a pyramid, and reassignment of every chat. This plan
sets implementation and qualification order without discarding current work,
changing scoring or inventing new package ownership. Existing in-flight grants
may finish; no foreign process, branch, staged file or claim is cancelled.

Architectural acceptance contracts are defined before implementation. Code and
evidence then grow bottom-up within each dependency cluster. This is incremental
integration, not a global waterfall requiring every low-level module to finish
before any consumer can be designed. Upper owners write contract tests, fixtures
and composition plans while providers implement; actual production admission
waits for the provider's required genuine boundary evidence.

## Layer and exit contract

| Layer | Responsibility | Evidence required before upward qualification |
| --- | --- | --- |
| L0 contracts | Schemas, canonical bytes, typed states/ownership, finite limits, compatibility and acceptance cases | Explicit provider/consumer contract, forbidden states and source/target/features scope; no fabricated implementation proof |
| L1 pure modules | Parsing, crypto composition, encoders, proofs, bounded state, deterministic algorithms, UI controller state | Scoped fmt first, units/negative/property/oracle tests as applicable, check/strict Clippy, input hashes and declared limits |
| L2 concrete boundaries | File/pipe/socket/native identity, FFI, child/reader lifecycle, durable writes and exports | Real controlled fixture tests on the actual supported OS/provider; observed owner retention, failure and cleanup; mocks remain mocks |
| L3 application services | Audit recovery, authenticated Authority, Runtime session/ACK/audit-before-effect, lifecycle and management | Provider/consumer contract tests plus narrow actual service integration, timeout/replay/partial-failure tests and invariant preservation |
| L4 host consumers | Framework/server adapters, CLI/service host, CEF/Meari UI and language ABI consumers | Actual supported host/install/process/UI pilot using qualified provider contracts, with source/binary/OS binding |
| L5 product and distribution | Docker product, adversarial catalog, cross-OS matrix, package/image contents, Pages and final main | Required real e2e/a11y/runtime/cleanup/artifact/document gates; source-bound failures/unsupported cases remain explicit |

These are module responsibilities, NOT six new crates or a mechanical Cargo
topological sort. Audit, browser and test-support each span pure and I/O modules.
The inspected locked/offline no-deps metadata records40 workspace packages and
their declared conditional path edges; it does not prove all feature resolutions
or execution. No dependency edge, facade split or directory-only rewrite follows.

## All23 chat responsibilities

Names below describe existing roles; app titles remain unchanged. One source
writer per actual claimed path remains mandatory. Work lanes and build-target
leases are independent; coding does not reserve an idle compiler target.

| Existing chat role | Owning work in the pyramid | Immediate completion slice and dependency |
| --- | --- | --- |
| Root integration | L0 shared graph/security contracts; upward integration | Signals/Integrity/Pass common ownership, graph/lock/toolchain, source-bound gates and complete roadmap; no per-sentence routine approval bottleneck |
| OS security and Windows | L1 typed binding; L2 bridge/process/credential ownership | Finish bounded-create and shared capture/direct-pipe source, pure regression and actual five parent cases, then publish one stable consumer API; no fake producer permit |
| Audit custody | L1 canonical/Merkle; L2 WAL/source; L3 recovery | Qualify owning native inspection facade, named capacity/FD profiles, then typed recovery; production Unsupported persists until accepted actual gates |
| Authority IPC | L2 peer evidence; L3 authorized decisions | Encoded-secret lifetime correction, provider contracts with native/audit, replay/approval/expiry and real IPC; do not own duplicate native primitives |
| Runtime Gate | L3 composition and enforcement | Consume qualified capture/audit/Authority/Pass contracts, single application-scoped temporal owners, actual request/ACK/origin and lifecycle tests; frozen Engine stays sole scoring authority |
| CLI installation | L4 service-host consumer | Consume shared direct-pipe owner; actual four watchdog boundary cases and installation/service contract; no private child-lifecycle fork |
| Browser Meari/CEF | L1 navigation/controller; L2 CEF boundary; L4 UI | Pure history/Stop/loading tests first, then native-ui/CEF build and fresh bounded GUI pilot; actual keyboard/AT/IME are distinct gates |
| Product lab | L1 strict receipt projection; L5 topology | One safe actor-outcome validator, decode before witness assertion, narrow boundary checks before genuine Docker composition; no raw-output disclosure or manufactured green |
| CI/CD | Supporting runner/tooling; L5 distribution | Fast scoped workflows and Rust evidence helper, native tar writer/reader contract, targeted genuine hosted profiles; OIDC environment only, no publish |
| Node/Python/Ruby adapters | L4 shared framework consumers | One shared adapter/Authority contract, actual host ABI/GC/GVL/ASGI/WSGI/Rack cases after lower primitives; no duplicate scoring or native readers |
| nginx adapter | L4 server consumer | Maintain exact qualified Linux tuple, test upstream contract changes, complete unsupported host/profile gaps without replacing proof with pure checks |
| Apache adapter | L4 native host consumer | Preserve three-MPM actual45 tuple; requalify affected native/Authority integration and cleanup on changed inputs only |
| Go-origin adapter | L4 Runtime consumer | Preserve bounded Go8 tuple, finish remaining approved framework tuples after Runtime prerequisites; no new Go core or Go SDK implementation |
| .NET adapter | L4 native/ABI/Authority consumer | Reclaim source lane and consume stable native binding/direct-pipe/producer owner, then actual Kestrel/GC/AOT/package cases; retain exactly three C# glue exceptions |
| Native ABI | L1 safe handle/buffer contract; L2 export boundary | Pure ABI ownership, generated header/compiler/dynamic-load and allocator tests before language host qualification; no broadened unsafe allowance |
| Network/capture | L1 MAD/rate/correlation/traffic; L2 transport | Qualify shared history primitives then producers and real capture epoch binding; runtime owns lifecycle, fake fleet backend is never readiness |
| Watermark | L1 payload/codecs; L2 real carriers | Preserve29 component proof, qualify standards-based WASM/PNG and high-level validated composition before actual resource/HTTP/Docker pilots |
| Model training/correction | L1 record/kernel/evaluation; L2 file/process lab | Finish parser real-file case, RNG compatibility and bounded kernel/evaluation, then isolated lifecycle; no production activation/threshold changes |
| Wargame | L1 legacy report renderer; L5 adversarial cases | Complete bounded report primitive/CLI and narrow genuine generic26 execution independently of browser/archive work; preserve unsupported7 and exact denominators |
| Performance | Cross-cutting measurement/tooling | Bounded shared preparation implementation and cache experiments at the touched layer; report fixed-corpus results narrowly, no unrelated production-code writes |
| Convention audit | Cross-cutting independent review | Prioritize security/lifetime/contract blockers; batch style/proof re-audits at slice boundaries, no duplicate fixes or source-lane occupation |
| Developer tools/M11 | Supporting ownership and preparation | Encode layer/gate/lease policy in existing Rust tooling and concise rules; no global writer-mode flip, forced claims or foreign shell implementation |
| Pages/Draft v0 | L5 public claims and standards | Consume completed source-bound slice receipts; publish maturity/unsupported limits, batch rendering at public-input milestones; no claim from triage alone |

## Active dependency clusters and immediate order

1. Native ownership cluster: binding/bridge/capture/direct pipes -> actual native
   boundary proof -> CLI/.NET/Authority consumers -> real supported service pilots.
2. Audit cluster: canonical/framing/Merkle -> native inspection/FD/capacity ->
   typed recovery/durable ownership -> Authority/Runtime -> product custody.
3. Request/security cluster: signals/behavior/Integrity/Pass/history -> frozen
   Engine and authenticated session contract -> adapters/Runtime -> browser e2e.
4. Resource/model cluster: bounded record/payload/codec/kernel -> actual file/
   carrier/process -> application integration. Model activation remains separate.
5. UI cluster: pure controller/history/epoch -> actual CEF host API -> fresh
   browser navigation/keyboard/IME/AT. Public UI state must match actual host state.

Independent clusters continue in parallel. A narrow vertical smoke test is run
as soon as a cluster's provider/consumer seam is ready; full product tests remain
mandatory at integration milestones rather than becoming a substitute for units.

## Operational changes derived from the bottleneck investigation

- Fmt and cheap contract checks precede expensive preparation/compilation.
- Resolve selected package/target/features and hash relevant inputs; do not scan
  unrelated Rust documentation or the entire user cache. Scope omissions remain
  explicit; required final qualification inputs are never waived.
- Keep one immutable source materialization per compatible batch and stable
  exclusive targets. Return execution slots at terminal commands before evidence
  serialization. Never overwrite another live snapshot or reuse stale hashes.
- A routine reversible correction within an already authorized lane/attempt class
  preserves the first failure, fixes owned source and starts a fresh recorded
  attempt with admission. It does not require repeated human approval. New graph,
  unsafe, scoring, privileged scope or hosted dispatch keeps its explicit decision.
- Each owner reports only contract change, ready-to-consume slice, actual start/
  terminal/failure or a real blocker. Root keeps the DAG/leases and merges shared
  graph changes; it does not reapprove every ordinary formatter/parser fix.
- Each accepted layer receipt names provider source, OS/target/features, exact
  tests, missing capabilities and consumer obligations. A lower-layer unit pass
  never certifies a higher-layer runtime, a mock backend or another OS.

## Research basis and limits

NASA's software test-plan guidance describes bottom-up integration as testing
lower modules first and progressively integrating higher modules. Google's test
pyramid guidance retains unit, integration and a smaller set of end-to-end tests.
The Practical Test Pyramid emphasizes narrow real-boundary integration and
provider/consumer contracts, rather than relying solely on broad UI tests.
No fixed test-count percentage is adopted; Humanymous needs substantial native,
browser and adversarial evidence at its real security boundaries.

Primary references reviewed2026-11-08KST:

- https://swehb.nasa.gov/spaces/SWEHBVD/pages/140640607/STP%2B-%2BSoftware%2BTest%2BPlan
- https://testing.googleblog.com/2015/04/just-say-no-to-more-end-to-end-tests.html
- https://testing.googleblog.com/2010/12/test-sizes.html
- https://martinfowler.com/articles/practical-test-pyramid.html

Main integration, branch retirement, package publication and existing security/
accessibility constraints are unchanged. Bottom-up order does not shrink the
original required feature set or permit declaring the goal complete at L1/L2.
