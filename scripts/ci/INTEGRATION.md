# Rust delivery follow-up integration

Owner: e2e-infra / rust-delivery-followup. Root owns common Cargo/lock/toolchain.
New implementations/tests here use Rust 1.99.1, edition 2024, literal process
argument arrays and no authored script fragments. Legacy Windows actor code is
owned by its migration owner, not silently copied into this tool.

## Root Cargo delta

Root integrated `scripts/ci/delivery` (package/bin `humanymous-delivery`) as one
of25members and resolved the common589-package lock. Locked16tests/Clippy and
policy now pass against that common authority.
Dependencies: existing serde_json 1.0, sha2 0.10.9, ureq =3.5.1, tar =0.4.46,
flate2 =1.1.11, toml =1.2.1, time =0.3.55, plus yaml-rust2 =0.13.0. Temporary
independent `[workspace]` and child lock are removed. The preliminary lock is
preserved privately with matching SHA256 for its earlier audit; it is not a
second committed authority. Container contexts also include the root-added
skills/agent-maintenance member, not only crates/docs/test/CI members.

## Entry points

`cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- COMMAND`.
Commands: policy, action-pins, jobs, readiness, package, native-artifacts,
image-sources, node, release, crate-waves, publish-wave N, publish-npm,
audit-closure, pilot native|browser|docker, snapshot, and
`observe-run RUN_ID EXACT_SHA PRIVATE_OUTPUT_DIRECTORY`.

The publication subcommands are implemented gates, not authority to publish.
publish=false, incomplete runtime/source parity, Go shipping images, absent
registry ownership/bootstrap/trust and stale docs remain blockers. No registry
setting, tag or package is modified by local validation.

## Owning runner interfaces

Native (all three OSes), owner Windows runner chat:
`cargo run --locked --manifest-path scripts/ci/windows-native-runner/Cargo.toml
-- native --report ABS --binaries ABS`. Actual actor identity and per-process
exits/hashes remain its responsibility. Exit 2 is blocked/component-only; 1 is
execution failure, 0 requires actual full qualification before CI can accept it.
Its Windows standard-user account/child lifecycle is separate from a normal
user native pilot and must replace the old runner-probe actor only after tested.
The CI aggregator selects standard-user only on hosted Windows; local native
does not create accounts. The same owner added workspace/standard-user-workspace
scope (humanymous.workspace-probe/1) for default-feature all-target accounting;
delivery `probe OS ARCH` validates actual tuple, invokes that scope, then uses
the owner's summarize API to publish only sanitized target/runner-probe/report.json.
The six-tuple/full CI calls are switched to this Rust path. Actual whole-workspace
and hosted-account execution still need the exact-source qualification; the
four-package native pilot stays separate. Legacy Python/PS probe files are no
longer CI consumers and await final producer parity evidence before retirement.

Docker/browser, owner Compose chat:
`cargo run --locked --manifest-path test/lab/runner/Cargo.toml -- run --evidence
EXISTING_LOCAL_BUNDLE --require-full [--collector]`. lab-report.json uses
humanymous.product-lab/2, source-manifest.json contains immutable input bytes.
Exit 0 scoped success is not full qualification; require-full exits 2 while
required_unavailable is nonempty. The CI tool does not duplicate Compose/network
or in-container client code. Browser collector component is not human/AT/Pass.

Docs, owner Pages chat: `cargo run --locked -p humanymous-pages -- validate
--source docs --destination ABS --baseurl VALUE`, twice for apex/subpath.
Source review hashes are never refreshed by CI. See docs/INTEGRATION.md.

## Source binding and failure semantics

Source snapshots record actual bytes, HEAD, dirty state and SHA-256 manifest.
They never normalize LF after hashing or substitute a different source's green
run. Missing owner runner/report/cases, unknown exit, non-passing required case,
mid-execution source change or component-only qualification all fail the full
gate while preserving actual component reports. No no-sandbox launch is added.

Run 42339951733 at f32042af9136560b41c1765f565c2e04fe373a22 completed failure.
Its exact terminal metadata/log/artifacts are in private delivery follow-up
evidence. Four of 16 jobs succeeded (policy/Pages/Node/MSRV); native three jobs
reached real build/tests/release build then refused missing native runner.
Other failures include product browser/Docker runners, Go shipping images,
publish=false packaging, stale docs inventory, Unix native-cli dead-code Clippy,
and root Trivy findings in frozen Go dependencies. These are retained as this
source's failures, never replaced by later component successes.

Actions observation/2 writes attempt-N directories, verifies the terminal marker
before writes, requests attempt-specific logs, and downloads exact artifact IDs
only for matching source/run and creation within that attempt window. Inventory
pagination fails closed. Existing observation/1 evidence stays unchanged.
Current local delivery tests:22 passed; strict Clippy passed. Docs-review:8
passed and strict Clippy passed, including generated-cache/tracked-source boundary.
Observation/2 live retrieval succeeded for42339951733 attempt1: same failure,
same original log hash, three exact artifact ZIPs in a separate private directory.
The later archive-hash marker addition passed its targeted test and Clippy. This does not
qualify the failed CI run or a full product.

Windows owner checkpoint16c9e89 adds exact captured-helper contracts for audit
durable_witness/key_backups; tool7 tests and Clippy passed. Its actual73-harness
workspace retry still rejects durable_witness9 nested child summaries despite
process exit0. Audit producer repair and a new exact-source execution remain
required. Separate source91f93 native release reports14/14harness67passed,
errors0,requiredignored1,blocked2; this does not qualify the whole workspace.

First free hosted42347732666/attempt1/bfc9f623 is terminalfailure:22deliverytests
andpolicy pass, token-authenticated upstream aquasecurity commitlookup HTTP403
orgIPallowlist. Raw private joblogSHA256
d190ced6d26602aaea7aa49d7e758c2cf847f148399ec1d6eebe4b75788ca0c3.
Follow-up public unauthenticated lookup preserves exactSHA+entrypoint assertions,
HTTPerrorsfailclosed,unique-action denominator and no retries. Corrected source
is separate from that failed run; root alone starts the next hosted batch.

## Current source checkpoint and required observation

Delivery checkpoint f82da60 passed 47 pure tests, strict all-target Clippy,
actual static policy, actionlint and scoped specification checks. Lab checkpoint
7b2a98c supplies the producer contract consumed by that delivery source. These
checks do not qualify native processes, containers, browser execution or images.

Run 42348518353 at 63f69ae9de703939b7b9091e2f215b12bea265de remains
terminal failure: 16 jobs, three successes, 13 failures, zero skipped. Its raw
whole-run log SHA256 is
5c03079b00af0704ec0387b82b27fc7e4178cd838a1737eb6f300863252d0e3c.
Its native harness causes and private owner transcripts remain unrecovered;
new source diagnostics cannot retroactively explain or qualify that run.

The two required hosted lanes now include separate collector and product-browser
profiles, the Nginx 35-scenario owner contract and a real runtime binary image
candidate. The candidate build/CVE job does not replace the legacy core/gate
source guard. Both legacy image sources still block shipping until reviewed
Rust replacement evidence exists. No image startup, actor behavior or platform
qualification is established by a Dockerfile inventory or successful build.

The sole public receipt vocabulary is test/lab/runner/public-receipts.json.
CI independently hashes the private lab source-manifest.json and
wargame-report.raw.json, checks the exact source binding and 26 ordered case
indexes, and recomputes terminal totals and scoped status. Only exact allowlisted
fields reach public summaries. A valid receipt survives surrounding full-product
failure; a malformed present receipt adds a CI error. Absent evidence stays null.
The raw original bytes and private actor/body fields are never printed publicly.

Optional archive transport is restricted to checksum-verified public crates.io
archives, trusted-push saves and read-only live free-quota/retention checks.
Cargo timing observations expose hashed inputs and explicit unknown reuse
dimensions. Neither mechanism bypasses a validation gate or proves a cache hit,
equivalent compiled artifact, native ABI or complete source qualification.

The next immutable hosted source must include the root Cargo graph/lock and all
producer inputs. The root chat owns its single push/dispatch. New required
process profiles, language hosts and Pages browser checks remain pending until
their actual owner invocation and source-bound hosted evidence are integrated;
default library tests and unsupported profiles cannot substitute for those runs.

Runtime candidate scanning now prepares an exact clean-source manifest before
build, binds it into the image label, and verifies typed Trivy 0.70.0 JSON
against the observed local image ID and both source labels. The required
rustbinary package inventory must describe the actual runtime entrypoint;
empty scans and Cargo.lock-only results remain not_scanned. Vulnerabilities,
scanner errors and modified findings block. Raw JSON remains private and safe
output retains only hashes/counts/status and explicit unknown completeness.
This gate does not add embedded inventory to the binary or establish shipping
qualification. Its 58 pure tests and Clippy are separate from actual scanning.
