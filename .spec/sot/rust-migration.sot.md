---
id: sot:root/rust-migration
title: Rust migration conventions
status: active
created: 2026-11-06T19:40:11Z
updated: 2026-11-07T04:56:14Z
verify:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --workspace --locked
rel:
  governs: [Cargo.toml, crates/**, AGENTS.md, spec.toml]
---

# Rust migration conventions

## Explicit console-preview acquisition

BrowserCollector.probe_cdp_json ports Error.stack/console.debug and prototype
Proxy ownKeys/console.groupEnd using Rust-authored managed callbacks and empty
synthetic objects. It is explicit, cached once per epoch and disabled after stop.
Attempted/unsupported/failed remain separate. After explicitly requesting these
observations, an incomplete attempt rejects snapshot rather than silently using
negative flags. No universal current-Chrome/CDP/human/code-attestation claim.
Native19/actualDockerFirefox51/strictWASMClippy pass in the mounted-WASM component
scope. Bootstrap/distribution/session/transport and full product e2e remain pending.

## Explicit forwarding hook receipt

BrowserCollector.enable_guard_hooks explicitly opts into global eval/Function
Proxy replacement and its direct-eval lexical compatibility delta. Default
construction does not replace globals. JS-owned managed closures preserve
retained proxy calls after stop/drop. Counter observation forwards opaque caller
arguments without authoring/storing script source. Current references alone are
restored, external replacements and descriptor changes preserved. One active
owner per realm in this WASM module avoids stale nested restoration. Actual
Firefox47/0/0 and strictWASMClippy pass, including partial rollback, identical
throw/return/receiver/newTarget, reentrancy and failed restoration. Default guard
remains partial; explicit hook coverage can report measured dynamic coverage,
never human/code attestation or qualified collector/session/transport readiness.

## Owned observer implementation receipt

Collector's shared guard Epoch now receives actual MutationObserver script-node
events, with bounded acquisition, queued-record drain, frozen stop/drop and inert
callback retention on disconnect failure. Current native19/actualFirefox40 and
WASM strictClippy pass; eval/Function coverage stays absent and reported=false.
Browser execution mounted the source-built WASM read-only into an existing
pinned container image; fresh image build and full product e2e remain separate.

## Provisioned adapter validation integration

Browser request binding0.1 is a separate domain, not native RIT compatibility
or collector/capture qualification. Pure shared integrity codec emits literal
humanymous.browser/0.1 followed by newline, then14 fields each decimal UTF8 byte
length, colon, exact value, newline: installation,tenant,origin_identity,revision,
authority,session,method,target,counter,bucket,deadline,content_type,
application_authorization,lowercase SHA256 of raw body. Counters are1..2^53-1;
bucket/deadline0..2^53-1. Identity128, authority1024, canonical32byte base64url
session43, origin-form ASCII target8192, content-type1024, authorization8192,
body1048576 bytes maximum. Header boundary trims only ASCII space/tab, rejects
remaining control/nonASCII, preserves internal spaces. Final Request bytes and
headers must be signed; redirects/retries/streams require separate contracts.
Runtime owns freshness, one-inflight/replay/epoch/deadline, Origin/FetchMetadata,
cookie-free opt-in and required producer qualification. Existing token primitive
signs canonical bytes plus big-endian counter/bucket, explicitly repeated domain
binding. Initial auth-only profile does not claim cookie-bearing compatibility;
route-scoped cookie prepare/identity binding remains required follow-up.

Shared integrity exposes parse_for with exact provisioned identity/version and
read-only observation getters. Legacy parse remains Node-only. Protocol0.1
schema/body/framing/provenance and null capture/stream restrictions stay fixed;
nginx-specific observations require explicit provisioned AdapterProfile::Nginx:
identity humanymous.nginx/0.1.0-experimental.1, nginx-raw or nginx-normalized,
always null connection ID/capture/stream. Normalized mode also requires null
peer and rawHeaderOrder=false. Raw mode means parsed ordered HTTP headers only,
not raw TLS/H2 or independently authenticated original client provenance.
Generic parse_for preserves its prior Node observation vocabulary; callers
must use parse_with_profile for the new known profile. Local integrity12unit+3legacy parity,
authority3unit and strict Clippy passed. The source-bound receipt is
adapter-provisioned-results.json in evidence:root/20261106235519.

Guard baseline capture now feeds partial measured fields into BrowserCollector
snapshots, sharing the pure Epoch rather than duplicating counters/lifecycle.
Native19 and actual isolated Firefox34 pass; native-toString text is transient
and bounded before copying. No eval replacement/Function hook/MutationObserver
is installed, so reported=false and frozen scoring consumes no new guard signals.
Guard stop is attempted alongside passive cleanup and freezes only on success.

Native CLI862883c is integrated as workspace member20: package
humanymous-native-cli, binary humanymous-native, init subcommand. Reuse existing
serde/serde_json/junction versions; publish=false and Rust1.99.1 remain. The owned
source/manifest hashes match evidence:root/20261107042614; root independently ran
locked11unit+6actual-process tests and Clippy. Windows explicit no-clobber JSON
publication and all-framework dryrun planning do not implement or qualify SDK
adapters. Other-OS writes, multi-file Rust scaffolding, doctor/verify and SDK/ABI
consumers remain later work. No generated foreign glue or installer hooks.

Browser1a7de24 adds a borderless Rust SVG/Meari shell and native resize/cursor
handling, plus accepted opt-in native SDK design. Final core18/native11 and
explicit rejection case1/flags4 are source-scoped; native96DPI gestures/readbacks
belong to the separate prior GUI source manifest. Later max-edge/DPI/capture fixes
are unit/build-only. Planned native SDK crates are not workspace packages yet;
that slice retained19; the later native CLI raises this to20. No sandbox,
navigation, Chrome parity, SDK ABI or full AT claim.

The guard migration starts with a pure bounded epoch/state API and four native
tests. It accepts event kinds/baseline counts only, refuses precision overflow,
requires hook coverage and freezes on stop. It installs no runtime hooks and does
not qualify browser guard.reported or session transport. The injector direct-eval
compatibility finding and browser acceptance gates are in sot:root/client-collector.

The collector adds an explicit Rust asynchronous first-party environment probe
with shared Environment wire type, owned DOM/timers/abort and bounded privacy
metadata. Its standalone measured API does not qualify snapshot/session transport.
Evidence: evidence:root/20261106235519/environment-collector-results.json.
Its subsequent asynchronous prerequisite shares one timer owner and distinguishes
fulfilled undefined, rejection and timeout; no duplicated per-probe timeout logic.
Advanced field collection now implements collectAdvanced through an explicit
Rust async API; native13 and actual isolated Firefox30 pass. Shared timer and
WebGL acquisition prevent duplicate implementations while preserving each old
probe's fallback scope. Guard, fingerprint-ID/ICE producers, distribution and
qualified snapshot/session transport remain required; bounded API tests are not
whole-runtime parity. Advanced capability absence is never a new hard rule.

## Collector implementation delta c3389b6

The root workspace now includes humanymous-collector, the seventeenth package.
Its pure17signal L1-L3 evaluator shares canonical constructors/registry and its
wasm32 browser boundary is authored Rust with pinned generated binding tools.
Native6 and actual isolated Docker Firefox3 tests pass in the recorded manifest.
The partial report does not complete L4/environment/guard/advanced/session/RIT
or full browser/product parity. See sot:root/client-collector and
evidence:root/20261106235519/collector-results.json. Detection freeze is unchanged.

Subsequent native integration registers humanymous-cef-bootstrap and
humanymous-cef-desktop, bringing root membership to19. The Windows native FFI
boundary retains local reviewed unsafe allowances under deny; shared core keeps
forbid. All publish flags remain false. Non-Windows empty native crates do not
establish cross-platform browser implementation. The independently locked Rust
scripts/ci/cef-pilot tool explicitly validates provisioned negative cases and
never substitutes them for GUI/human/AT or full sandbox evidence.

## Rule

Native custody and WAL now share one crate-private storage::ExclusiveFile owner
using standard File locking and explicit unlock-on-Drop. There is no production
clone/unlock handle API. fs4 is removed from the root and runtime-pilot locks;
same-inode and successor-exclusivity regressions cover the common mechanism.
Unix authority denial-helper execution is checked explicitly and its Clippy
issue is repaired without ignored-test or permissions exceptions.

The passive browser behavior port extends humanymous-collector with one bounded
pure Recorder and owned passive WASM listeners. Existing JS reducers are unchanged
cfg(test)-only differential oracles in the component lab, not new authored code
or product glue. Retire them only with validated replacement golden evidence at
the final foreign-source gate. No scoring/a11y policy changes or full-runtime
qualification follow from this component result.

Port the complete product to a Cargo workspace on `codex/rust`. Preserve observable
behavior, scoring weights, hard-rule predicates, verdict bands, signal IDs and
wire formats. The Go implementation remains the differential oracle until each
context is proven equivalent. A Rust wrapper around Go is not completion.

### Structure and naming

Current program status and follow-through are canonical in
[[sot:root/program-roadmap]]. The latest Rust-authored-code requirement includes
new application UI/adapters/tests and the disposition of existing executable
foreign-language scripts, including script strings embedded in Rust. Do not
classify those as Rust merely by container file extension. Existing non-Rust
prototypes remain explicit migration debt until reconciled.

The laboratory-browser UI uses user-selected Meari. The user subsequently
authorized core MSRV and pinned toolchain 1.99.1 as well. Keep desktop dependency
and integration boundaries explicit without retaining an obsolete 1.88 baseline.
Chromium/CEF is an external native engine dependency, not a Rust implementation.

- Each crate has one bounded-context responsibility, a kebab-case package name,
  `Cargo.toml`, `src/lib.rs` or `src/main.rs`, and a small public API.
- Use standard Rust snake_case modules/functions/variables, PascalCase types and
  traits, and SCREAMING_SNAKE_CASE constants. Public APIs have rustdoc contracts.
- Group files by domain context and responsibility. Do not copy the flat Gate
  catalog into a directory of unrelated Rust files or make a catch-all utility
  module. Use request, credentials, administration and configuration submodules.
- Composition roots only wire services. Detection owns verdict computation;
  enforcement consumes it and never duplicates scoring.
- Do not duplicate features, create parallel implementations inside Rust, or
  fragment cohesive code into forwarding-only modules. Each responsibility has
  one owning implementation and one contract; all consumers reuse it. Extract
  shared code when repeated semantics justify it, within the context that owns
  those semantics. File/crate boundaries need a responsibility or dependency reason.
  Standard lib.rs/mod.rs namespace facades are allowed; redundant forwarding
  functions and artificial modules without a boundary responsibility are not.
- Keep pure transforms separate from I/O, clocks and randomness. Inject these
  dependencies at boundaries. Avoid unsafe code unless separately justified.
- Unit tests are mandatory for every implemented responsibility and negative
  boundary. Put unit tests beside the implementation, integration tests under
  crate `tests/`, fixtures under a context-specific fixture directory.

### Non-negotiable parity

1. Preserve all existing scoring fixtures and regression cases, including Core
   versus Gate premises. Add differential boundary tests; six freeze goldens alone
   do not establish full-engine parity. Never regenerate expectations to hide drift.
2. Capture raw ClientHello, ordered HTTP headers, HTTP/2 SETTINGS and priorities
   before frameworks normalize them. Prove raw-socket rejection/forwarding parity.
3. Preserve HMAC domain separation, UTF-8 canonicalization, big-endian integers,
   base64url padding rules, constant-time comparisons and credential expiry/binding.
4. Audit canonicalization must preserve NFC, byte lengths, sorted collections,
   decimal formatting and empty fields. Existing WALs and proofs must still verify.
   The baseline Go 1.26.3 selects x/text normalization Unicode 15.0.0; x/text's
   alternate Unicode 17 table is selected only by Go 1.27+. Pin the baseline
   behavior, including stream-safe CGJ insertion, rather than using an arbitrary
   newer normalizer. Normalization-version compatibility needs explicit vectors.
   Valid Merkle roots/proofs stay byte-identical. Invalid proof framing may be
   hardened fail-closed: reject extra inclusion-path nodes and malformed hash
   widths, including fabricated roots for structurally impossible paths.
   Audit duplicate signal IDs retain the Go 1.26 sort.Slice permutation; a
   stable Rust sort is not historical hash parity. The narrow Rust compatibility
   implementation retains the upstream BSD notice. Array rule/verdict bytes
   preserve their source behavior instead of applying extra normalization.
   Witness hardening rejects a replayed older head even if previously observed,
   a changed Merkle root at the same size, missing/malformed roots, wrong node
   or invalid writer signature. Restore authenticates the prior witness signature;
   deployment still needs independently durable latest-head/anti-rollback state.
5. Missing required signals, replay, invalid signatures/settings and storage
   outages must retain fail-closed behavior. Erasure, kill switch and permanent
   bans retain dual control. Never gate Pass on motor richness or speed.
6. Rust WASM must preserve browser exports and collector behavior; update its
   loader and remove Go runtime dependency only after real-browser comparison.
7. Preserve T4 ceiling and defense-only lab isolation. Measured catalog results
   must never become claims of universal bot detection.

### Records and documentation

All new specifications, decisions, execution records and evidence covers belong
to Specfold documents. `.spec/` stores reviewable canon; `.spec.local/` stores
private working notes and raw local evidence and is gitignored. Do not copy keys,
tokens or personal data into either tier. Historical SoT references remain
provenance; unavailable `sots/` contents must not be invented or claimed imported.

Rebuild public docs as authored Markdown using pinned
`remote_theme: modootoday/jekyll-theme-modootoday@<reviewed tag or commit>` and
standard GitHub Pages configuration. Do not generate documentation content using
the legacy docsgen scripts. Migrate existing design tokens to the theme's supported
token overrides. Validate source and built site with the theme checker. GitHub
Actions must enforce documentation freshness before releases and build/test across
Linux, macOS and Windows. Design and implement crates.io Trusted Publishing through
OIDC, preserving least-privilege job permissions and protected release environments.
No long-lived crates.io token is a substitute for the requested OIDC release path.
Documentation must describe verified shipped behavior, with English public prose.

### Completion and integration

Audit all final Rust runtime/build/test dependencies, Node, Ruby/theme and
container bases against current vulnerability databases. Resolve reported
CVE/advisory findings with maintained fixed versions and rerun the relevant
tests/scans. A required scan that did not run is not a clean result. Record
advisory IDs, affected paths, fixed versions, database time and residuals in
Specfold evidence. Prefer vetted maintained crypto over custom cryptography;
any replacement implementation requires its own normative contract and tests.

This session targets the new Rust branch, not maintenance of the old Go product.
Keep Go source/dependency manifests frozen as the migration oracle; its baseline
vulnerability findings are provenance, not a Go patch backlog. Remove Go from
final shipped build/deployment paths as Rust replacements pass their gates.

Every Go source and test responsibility must be mapped to implemented Rust code
and evidence. All binaries, WASM, Docker images, Compose, CI/release paths and
public adapters must ship Rust. Pilot/restart/replay/failure tests and the complete
Docker attack, Gate, Pass and swarm suite are mandatory; host runners do not count.
Run formatting, Clippy with warnings denied, workspace tests and WASM builds.

Only after all gates pass, acquire the git mutex, fetch shared main, integrate it
into the migration, rerun gates, merge to shared main without force and publish.
Delete the completed migration branch locally/remotely after confirming main
contains it. Do not delete unrelated contributors' branches. No partial migration
may be called complete or merged as the final Rust replacement.

Before that final merge, remove the Go product sources, obsolete Go build/WASM
runtime paths, migration-only Go generators, retired documentation generators,
unused legacy assets/configuration and stale CI/deployment references. Retain
required frozen fixtures and provenance in their canonical Rust/Specfold homes.
Optimize .gitignore, Docker build contexts and Cargo package include surfaces:
private .spec.local evidence, caches, local credentials, generated outputs and
the defense lab must not leak into public runtime/package artifacts. Verify the
cleaned final tree through the complete gates before merging; cleanup is not an
untested post-merge action, and published Git history is not rewritten.

## Integration adapters and agent surfaces

The expanded Cross-OS/Cross-Platform/Cross-Lang development scope, architecture,
target-family catalog and conformance gates are canonical in
[[sot:root/adapter-ecosystem]]. Adapters must satisfy that contract in addition
to the migration invariants below.

Include systematic Rust attacker/defender wargame simulation crates and an
extensible bypass regression catalog. Targets are restricted to this project's
isolated deployment; third-party probing is prohibited. Keep attacker artifacts
out of public defender runtime images and normal installation. Record attack
attempts, errors, skips, verdicts and human control outcomes without excluding
failures from denominators or calling CHALLENGE an ALLOW. Docker full-lab pilots
must demonstrate reproducible attack-to-enforcement evidence. Wargame expansion
does not authorize changes to frozen scoring weights, predicates or verdict bands.

Deliver a Rust humanymous CLI for supported operating systems and server
environments. Include doctor diagnostics and install --dry-run planning before
installation: dry-run must not modify files, services, privileges or network
configuration, and actual install must report exact managed changes and support
safe repeat execution. Validate native OS detection, prerequisite failures,
configuration conflicts, local-only management binding, rollback/error handling
and generated service configuration through unit tests and real OS pilots.
Ship CLI artifacts through the same verified cross-OS release pipeline.

The engine uses one stable combination implementation for scoring and traces,
following the existing Go trace path's first-seen group/layer ordering. Frozen
finite policy values, caps, weights, verdict bands and promotion predicates are
unchanged. Non-finite numeric policy/contributions return a typed error; callers
must fail closed rather than manufacture an ALLOW verdict from invalid arithmetic.
This input-validity boundary does not authorize scoring-policy changes.

The final Rust delivery includes a Node/Bun adapter package supporting Express
and Hono, published to npm through GitHub Actions trusted OIDC publishing, and
an nginx plugin adapter. Adapters activate the shared security and management
capabilities without duplicating the Rust policy engine. Management access must
be local-only by default; forwarded client headers cannot establish local trust.
Verify header validation, security-layer insertion, route reservation/collision,
streaming and existing application behavior in unit, pilot and Docker e2e tests.
Document capability and transport boundaries honestly rather than claiming all
features are automatically available through middleware alone.

Before final cutover, consolidate development instructions into AGENTS.md and
.agents/{rules,skills,agents}, rewriting them for the Rust development workflow.
Remove obsolete .gemini, .codex, .claude, .grok, .cache and .agent-rules trees
after preserving necessary evidence under .spec/.spec.local. Stop generating
vendor mirrors. User-facing Agent Skills belong in root skills/, each in its
own context directory with a standards-compliant SKILL.md and only necessary
supporting resources; keep them separate from developer-only .agents/skills.

All final skills require measured evaluation and improvement with modoo.today
skills' modootoday/skill-factory/codex-plugin-eval. Retain paired with/without
results, activation evidence, case-level graders and review of failures under
.spec.local, with a canonical acceptance summary in .spec. A successful syntax
check or ungraded/null result is not performance acceptance. Install the complete
skill-factory package because the evaluator imports shared resources; use its
Linux isolation requirements and bounded authenticated evaluation configuration.

## Why

The user requested complete Rust refactoring with mandatory modularity, tests,
Specfold canon and a fresh documentation site. Existing AGENTS hard constraints
remain binding. Source analysis found 26 internal contexts and 804 Go test
functions. Gate, network capture and audit continuity dominate migration risk.

The design panel and adversarial review identified the parity requirements above
as ship blockers. SRE requires real deployment/restart evidence; privacy requires
audit/erasure continuity; evaluator requires honest measured claims. No new
scoring policy is authorized by a language refactor.

## Verification

- `spec check` validates the canonical document graph.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace --locked` validate Rust slices.
- `go test ./...` preserves the oracle until cutover.
- `bash scripts/e2e-docker.sh` is the mandatory full-lab authority after Rust wiring.
- Module inventory and execution records track remaining work explicitly.

## Audit storage recovery boundary

JSONL recovery is bounded per frame including its newline. Fully framed corrupt
records are fatal. Only an incomplete final frame of the terminal segment may
be discarded, with truncation synced before success. Recovery reports loss and
never constitutes hash/HMAC/checkpoint verification. The caller must own the
writer lock and a service-controlled directory; symlink rejection alone does
not establish safety against concurrent hostile directory replacement.

The native WAL holds a cooperative OS lock for its lifetime; mutation requires
exclusive Rust access. Each append must write all framing bytes and sync before
success. Any disk error poisons the writer until restart. Segment indices are
contiguous and bounded to the ten-digit filename domain. Missing checkpoints
are explicitly distinct from independent anchoring. Native permissions and
cross-process/power-loss validation remain deployment gates.

Startup authentication must reuse the audit verifier and require expected node,
nonempty HMAC key and trusted writer key from installation state. A required
witness implies nonzero witnessed checkpoint coverage. Checkpoints must strictly
grow and include Merkle commitments. An independently retained trusted head must
be present exactly before restoring chain state. Empty history is valid only for
an explicitly authorized first installation without an existing trusted head.
No local verification can prove preservation of an externally unanchored tail.

Durable Log must authenticate recovered state before append, persist records
before advancing the chain, and persist due checkpoints before reporting
success. Any disk failure poisons the writer. Emit-and-act releases its closure
only after these boundaries; a durable record followed by checkpoint failure is
an ambiguous logical operation requiring explicit recovery, never blind retry.
UUIDv7 uses OS entropy and fails closed if entropy is unavailable. Local writer
signatures are not independent witness anchoring.

Configured witness co-signing must fail closed: no writer-only fallback after
refusal. Restore must verify persisted witness signatures and must not reset an
already-known witness head to empty/older local storage. Reference in-process
co-signing is separate from actual independent authority and durable antirollback
state; checkpoint cadence limits independent anchor coverage explicitly.

Audit lifecycle classification preserves cumulative inclusive retention windows;
no tier label claims physical archival or erasure. Liveness requires an observer
outside the watched process, expected-node provisioning and authenticated/replay-
checked receipts before state transition. The state registry does not authenticate
messages. Invalid cadence, duplicate registration or observer-clock regression
must not reset deadlines or return a false healthy indication.

Proof export must distinguish unsigned current/prefix roots from exact persisted
signed checkpoints. Verification reuses the shared Merkle implementation and
requires separately trusted writer/witness keys. A required witness must anchor
the first durable record immediately before periodic cadence begins, preserving
strict recovery after an early restart without relaxing witness coverage.

Sealed key compatibility preserves the frozen scrypt/AES256-GCM format while
rejecting malformed key widths, noncanonical envelope encodings and ordinary
empty-passphrase operation. Any authentication/parse failure must return an
error, never mint replacement identity or silently empty a linkage vault.
Compatibility import, physical durable storage and dual-control erasure are
separate gates. Owned-buffer zeroization is not proof of backup/OS erasure.

Per-subject linkage vault restore must reject duplicate keys, invalid widths,
key/tombstone conflicts and dangling reverse handles. Shredded subjects cannot
be reminted. Derivation and key destruction serialize on one state lock. KDF
failure never falls back to a cheaper algorithm. Exposing erasure requires a
verified dual-control permit and durable persistence of the resulting snapshot;
a test-only permit does not constitute an operational approval workflow.

### Native runtime process Docker pilot

The lab runtime-process profile executes the real humanymous-runtime binary with
private piped startup/readiness, actual durable audit storage and an owned
loopback origin inside one network-none non-root container. Its Rust pilot
reuses runtime canonicalization and integrity signing, never copies a detector.
Positive fixtures are client assertions, not human/browser qualification.
Measure origin effects for valid requests and zero extra effects for replay,
body mutation, missing session/report, reserved routes, unsupported capture and
ambiguous framing. Check credential stripping, duplicate Set-Cookie preservation,
authenticated audit recovery and session invalidation after process restart.
Report unsupported collector/TLS/H2 and independent origin isolation explicitly;
this bounded pilot does not complete full Core/Gate/browser/AT e2e.
