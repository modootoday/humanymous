# Third-party licenses

humanymous is distributed under the [Apache License 2.0](LICENSE). The distributed
binaries and container images statically link the Go modules below, and the web assets
include Go's `wasm_exec.js` loader. Each is a permissive licence compatible with the
Apache License 2.0; their copyright and permission notices are reproduced here as
required, and this file ships alongside `LICENSE` and `NOTICE` in every release binary
and image.

For the full verbatim licence text of any module, see its source repository (linked)
or the copy in your Go module cache (`go env GOMODCACHE`).

## Go standard library & WASM loader — BSD-3-Clause

> Copyright (c) 2009 The Go Authors. All rights reserved.

Applies to the statically-linked Go runtime/standard library and to
`web/js/wasm_exec.js` (redistributed verbatim). Source: <https://go.googlesource.com/go>
· Licence: <https://go.googlesource.com/go/+/refs/heads/master/LICENSE>

## Distributed Go modules

| Module | Licence | Copyright | Source |
|--------|---------|-----------|--------|
| `github.com/refraction-networking/utls` | BSD-3-Clause | Copyright (c) 2016 The uTLS Authors / Google Inc. | <https://github.com/refraction-networking/utls> |
| `golang.org/x/crypto` | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/crypto> |
| `golang.org/x/net` | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/net> |
| `golang.org/x/text` | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/text> |
| `golang.org/x/sys` (indirect) | BSD-3-Clause | Copyright (c) 2009 The Go Authors. All rights reserved. | <https://cs.opensource.google/go/x/sys> |
| `github.com/andybalholm/brotli` | MIT | Copyright (c) 2009, 2010, 2013-2016 by the Brotli Authors. | <https://github.com/andybalholm/brotli> |
| `github.com/klauspost/compress` | BSD-3-Clause (+ Apache-2.0 / BSD-2 for vendored parts) | Copyright (c) 2012 The Go Authors / Copyright (c) 2019 Klaus Post | <https://github.com/klauspost/compress> |

The MIT and BSD licences require that the copyright and permission notice be reproduced
in redistributions; that requirement is satisfied by shipping this file with the
binaries and images.

## Test-only (NOT part of the distributed engine or Gate)

| Package | Licence | Notes |
|---------|---------|-------|
| `playwright-core` (npm) | Apache-2.0 | Drives the red-team catalog in `test/redteam/` only; excluded from images via `.dockerignore`. |
