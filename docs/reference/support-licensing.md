---
title: Support, licensing & open-source notices
---

# Support, licensing & open-source notices

**Diátaxis quadrant:** Reference. **Audience:** buyers, legal, and procurement teams evaluating humanymous Gate before or during adoption.

humanymous Gate ("Gate" after first mention) is the reverse-proxy enforcement layer distributed in this repository as a **reference implementation, not a production-hardened build.** This page states plainly what the reference repo ships with respect to licensing, what third-party open-source software it depends on, and where to get help — and it marks the facts you must confirm for your own procurement, rather than asserting anything the repository does not actually contain.

For install prerequisites, see [Install requirements](./install-requirements.md). For how to report a security vulnerability, see the [Security disclosure policy](./security-disclosure.md).

---

## Project license (humanymous Gate itself)

**Apache License 2.0.** humanymous is released under the Apache License, Version 2.0; the full text is in the [`LICENSE`](../../LICENSE) file at the repository root, accompanied by [`NOTICE`](../../NOTICE) and [`THIRD_PARTY_LICENSES.md`](../../THIRD_PARTY_LICENSES.md).

The Apache License 2.0 is a permissive license: you may use, modify, redistribute, and build on the source in open or closed products, provided you retain the copyright notice, the license text, and the `NOTICE` attributions, and state significant changes you make. It adds two things a BSD/MIT license does not: an **explicit patent grant** from each contributor (with defensive termination if you initiate patent litigation over the software), and an explicit statement that it grants **no trademark rights** — relevant for a detection engine that embodies potentially patentable techniques and carries a distinct brand. It is compatible with the project's own third-party dependencies, which are BSD-3-Clause and MIT (below).

> **Note:** If you redistribute a build, keep the `LICENSE` and `NOTICE` files with it and carry the attribution bundle covering the third-party modules listed below. The Apache License does not license the "humanymous" name or logos; see [`TRADEMARK.md`](../../TRADEMARK.md).

---

## Third-party open-source notices

Gate is a pure-Go module (`github.com/modootoday/humanymous`) built with Go 1.25.3, no CGO. Its Go dependencies are declared in `go.mod`. The table below lists them at the exact versions the reference pins.

These modules are all permissive, BSD/MIT-family open-source software. Each SPDX identifier below was read from that module's own `LICENSE` file in the module cache.

| Module | Version | Dependency type | SPDX license id |
|--------|---------|-----------------|-----------------|
| `github.com/refraction-networking/utls` | v1.8.2 | direct | BSD-3-Clause |
| `github.com/andybalholm/brotli` | v1.0.6 | indirect | MIT |
| `github.com/klauspost/compress` | v1.17.4 | indirect | BSD-3-Clause |
| `golang.org/x/crypto` | v0.54.0 | indirect | BSD-3-Clause |
| `golang.org/x/net` | v0.57.0 | indirect | BSD-3-Clause |
| `golang.org/x/sys` | v0.47.0 | indirect | BSD-3-Clause |
| `golang.org/x/text` | v0.40.0 | indirect | BSD-3-Clause |

> **Note:** All seven are permissive licenses that allow redistribution with attribution. If you redistribute a build, generate an attribution/`NOTICE` bundle from these modules' `LICENSE` files to satisfy their notice terms. Re-confirm the identifiers whenever you bump a dependency version.

### Red-team harness (test) dependencies

The defensive red-team harness under `test/` uses one npm package, `playwright-core` (declared in `test/package.json`, installed via `npm install` in `test/`), which is **Apache-2.0** licensed. It is used only by the local validation harness, not by the Gate runtime binary.

> **Note:** If you bump the harness or add npm packages, re-read the licenses from the installed `node_modules` and record any that a redistribution would ship.

---

## Getting help

Support channels for humanymous Gate are **deployment- and vendor-specific**. The reference repository does not bundle a support contract, a hosted issue tracker, or a community forum; whichever of these you have depends on the vendor or operator that supplied your build.

| Channel | Where |
|---------|-------|
| Commercial / operator support | &lt;support-contact&gt; |
| Issue tracker | &lt;issue-tracker&gt; |
| Community forum | &lt;support-contact&gt; (if offered) |

> **TODO(verify):** Fill in the real `<support-contact>` and `<issue-tracker>` values for your deployment. Confirm whether commercial support and/or a community forum are offered for your build.

### Reporting a security vulnerability

Do **not** use general support channels to report a suspected vulnerability. Follow the coordinated process in the [Security disclosure policy](./security-disclosure.md) instead.

---

## Related pages

- [Security disclosure policy](./security-disclosure.md) — how to report a vulnerability responsibly.
- [Install requirements](./install-requirements.md) — build and runtime prerequisites for the reference.
