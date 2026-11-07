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
