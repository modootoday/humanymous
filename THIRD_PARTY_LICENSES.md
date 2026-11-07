# Third-party licenses

humanymous's project license is [Apache License 2.0](LICENSE). Third-party code
retains its own notices. This document preserves the historical Go distribution
inventory; it is not a manifest of dependencies in current Rust candidate binaries,
containers or browser bundles.

Current Rust candidates require artifact-specific dependency and notice review,
including native and upstream generated components. Audit contains a Go-adapted
sorting implementation whose complete notice is retained in
[crates/audit/LICENSE-Go.txt](crates/audit/LICENSE-Go.txt). The known-notice
packaging work does not establish a complete Rust dependency notice inventory or
verified contents of every archive/image. Check exact source-bound distribution
receipts before making an artifact claim. This document's presence alone does
not establish that every required notice was included.

The historical module source links below remain provenance references. Their
license texts can be inspected at the linked source or in the original Go module
cache; Go runtime or wasm_exec.js inclusion is not implied for Rust candidates.

## Rust model RNG compatibility source — BSD-3-Clause

The model RNG algorithms and tables translate Go 1.26.3 math/rand into Rust.
Copyright 2009 The Go Authors. All rights reserved. The complete notice is
retained in [the RNG license](crates/sdk/humanymous-core/src/model/training/rng/LICENSE).
Frozen upstream source snippets in the core model test fixtures retain the same
license. The source identities are recorded in the model RNG provenance evidence.
This attribution covers those translated sources and snippets; it does not prove
complete dependency or notice contents for a particular distributed artifact.

## Historical Go standard library & WASM loader — BSD-3-Clause

> Copyright (c) 2009 The Go Authors. All rights reserved.

The historical Go distribution used the runtime/standard library and
`web/js/wasm_exec.js` loader. Source: <https://go.googlesource.com/go>
· Licence: <https://go.googlesource.com/go/+/refs/heads/master/LICENSE>

## Historical Go distribution modules

| Module | Licence | Copyright | Source |
|--------|---------|-----------|--------|
| `github.com/refraction-networking/utls` | BSD-3-Clause | Copyright (c) 2016 The uTLS Authors / Google Inc. | <https://github.com/refraction-networking/utls> |
| `golang.org/x/crypto` | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/crypto> |
| `golang.org/x/net` | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/net> |
| `golang.org/x/text` | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/text> |
| `golang.org/x/sys` (indirect) | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/sys> |
| `github.com/andybalholm/brotli` | MIT | Copyright (c) 2009, 2010, 2013-2016 by the Brotli Authors. | <https://github.com/andybalholm/brotli> |
| `github.com/klauspost/compress` | BSD-3-Clause (+ Apache-2.0 / BSD-2 for vendored parts) | Copyright (c) 2012 The Go Authors / Copyright (c) 2019 Klaus Post | <https://github.com/klauspost/compress> |

This table preserves module identities and license labels; it does not replace
their complete license texts or prove the contents of a particular distribution.

## Historical test-only inventory

| Package | Licence | Notes |
|---------|---------|-------|
| `playwright-core` (npm) | Apache-2.0 | Used by the historical catalog in `test/redteam/`; current Rust artifact contents require their own inventory. |
