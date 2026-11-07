# Root integration

Register crates/go-adapter as a workspace member and update Cargo.lock under the
root Cargo slot. Dependencies reuse runtime and serde workspace declarations.
No Go producer profile, new authority transport or unsafe/foreign glue is needed.

Source currently uncompiled. Please assign the one local Cargo slot for locked
pure units, package fmt and scoped Clippy. Actual processes, Go HTTP hosts and
Docker product verification must run on standard free hosted runners.

Root integration commands after registration (not executed by this owner yet):

```
cargo test --locked -p humanymous-go-adapter --lib
cargo fmt -p humanymous-go-adapter -- --check
cargo clippy --locked -p humanymous-go-adapter --all-targets -- -D warnings
```

Manifest/dependencies frozen at root request while workspace integration waits
for the current wargame Cargo slot. Direct rustfmt and owned spec checks passed.

The crate is a deployment contract and receipt qualifier, not a Go SDK or proxy.
No tuple is qualified. It transfers validated runtime Configuration into the
existing composition root. Receipt structs never execute tests or authenticate
evidence. Hosted integration must supply source/image/external-fixture digests
and independent origin contact counters for all eight cases, retaining failures.
Corpus version2 replaces misleading origin_effects with origin_contacts and
requires handler_response_verified for the two positive cases (false for all
negative cases). Contact counts alone cannot prove binary echo or HEAD/cookies.
Version1 receipts do not qualify; this corpus does not measure application
mutations, human/browser provenance or 204/304/streaming support.
ExpectedTuple additionally binds host/Go version, actual OS/architecture and
sanitized configuration/frozen-engine digests. Expected values must come from
the trusted runner independently of receipt contents. Empty or stale values fail.

Pinned untouched candidate sources are in adapters/go/upstream-fixtures.json.
Their entrypoint SHA256 values were computed from GitHub API decoded bytes;
they are not full fixture/dependency/build artifact identities. Most hello-world
examples cannot meet the full corpus. No upstream Go file was authored or copied
into this checkout, and no native Go host bridge is implied.

Public docs delta for docs owner: distinguish net/http/chi/Gin/Echo from pooled
Fiber/fasthttp; universal Gate deployment does not imply native method-set,
ResponseWriter/hijack/cancellation compatibility. No native Go support claim.

## Handover: execution authorization gate

Mission remains complete Go HTTP Gate integration, all named host contracts,
owned unit/lint results and actual hosted pilot/product evidence. No detection,
accessibility, fail-closed or T4 rule changed. Read AGENTS.md and
.agents/lessons/HARD-WON.md before resuming; this chat is Codex-owned.

Done: source307e9cc, contact-proof correction ddda924, hosted build recipe31d9f69,
English-only authored prose, direct Rust formatting, LF worktree/index and scoped
Specfold check. Root reported two units for the older307e9cc source; that is not
evidence for the current three-test corpus. Native SDK/foreign glue is absent.

Blocked: no explicit current Cargo slot for this owner and no live Go-specific
hosted execution handle/result. Root and lab chat activity alone does not prove
a Go compiler/test job is running. Local Cargo and actual local processes/Docker
remain unauthorized. The runtime pilot now declares the Go path dependency;
its Go execution path and source-bound result still need implementation/validation.
Do not count an unexecuted recipe or synthetic receipt as pilot completion.

Next: root assigns a current pure-test/lint slot or runs the exact commands above.
Then product-lab consumes Deployment in the real runtime construction and checks
Receipt against independently selected ExpectedTuple; build the untouched fixture
with compiler-input.json/Dockerfile.fixture on the free hosted runner, retain
full fixture/binary/image/config/engine hashes and all eight case denominators.
Keep synthetic session HTTP compatibility separate from browser provenance,
contact counters separate from mutations, and non-net/http tuples NOT_RUN until
their actual lifecycle corpus is complete. Root owns push/dispatch/integration.

Read first: .spec/sot/go-adapter.sot.md, owned timestamp plan,
adapters/go/pilot-contract.json and compiler-input.json. Reclaim adapter-go before
writing after handover; use the global git mutex for any attributed owned commit.
No changes outside the claimed lane or common Cargo/lock/toolchain are authorized.
