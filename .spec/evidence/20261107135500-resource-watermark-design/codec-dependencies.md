# Codec dependency candidates for root review

Read-only primary-source research on2026-11-07. This is a proposal, not dependency
adoption, a locked graph, security clearance or permission to implement codecs.
The initial payload/ledger manifest remains frozen with existing dependencies.

## Registry identity

The official crates.io API's max_stable_version record supplies each version,
archive checksum, license, declared MSRV and yanked state below. No crate archive
was downloaded or executed; checksums are registry assertions, not locally
verified archive hashes. All four selected records reported yanked=false.

| Candidate | Version | Declared MSRV | License | Proposed role |
| --- | --- | --- | --- | --- |
| png | 0.18.1 | 1.73 | MIT OR Apache-2.0 | Lossless PNG decode/encode without the broad image package |
| crc32fast | 1.5.4 | 1.63 | MIT OR Apache-2.0 | IEEE CRC for historical frames and PNG chunks |
| data-encoding | 2.11.1 | 1.48 | MIT | Padded RFC4648 base32 for historical markers |
| wasmparser | 0.263.0 | 1.88.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Actual WASM structural and instruction validation |

Exact registry archive checksums:

- png0.18.1:60769b8b31b2a9f263dae2776c37b1b28ae246943cf719eb6946a1db05128a61
- crc32fast1.5.4:5f97d3416d5337d15c4b0d78c8c91df922ca143f57ec20e2d1c188333047ef04
- data-encoding2.11.1:4583a4551df46e2792f82ceeac45e850d2e2d5debba0b91f102385cda5b11f06
- wasmparser0.263.0:4f5fa0a054ae153606df820eb0a84213e28cda41bbe04ba2628e71b6ba100fcb

Primary registry records:
[png](https://crates.io/api/v1/crates/png),
[crc32fast](https://crates.io/api/v1/crates/crc32fast),
[data-encoding](https://crates.io/api/v1/crates/data-encoding),
[wasmparser](https://crates.io/api/v1/crates/wasmparser).
The exact-version dependency records use the same API with /VERSION/dependencies.

## Features, graph and correctness constraints

Propose png default features without zlib-rs/unstable. Its non-dev direct edges
are bitflags^2.0, crc32fast^1.2.0, fdeflate^0.3.3, flate2^1.0.35 and
miniz_oxide^0.8 with simd. This is not the full resolved transitive graph. Root
must inspect actual locked versions/licenses/advisories before adoption.
crc32fast defaults to std and depends on cfg-if^1.0. data-encoding defaults to
std (including alloc), with no normal direct dependencies in the inspected
record. wasmparser's proposed minimal features are std,validate,features, with
default-features=false to avoid unnecessary serde/component-model/hash-collections.
Its nonoptional normal edge is bitflags^2.4.1; optional edges hashbrown,indexmap,
semver,serde depend on selected features. Exact lock resolution remains root-only.

[PNG decoder documentation](https://docs.rs/png/latest/png/struct.Decoder.html)
identifies its memory limit as best-effort and excludes caller-allocated output.
Therefore png::Limits alone cannot prove the accepted hard128MiB working-memory
cap. Before PNG admission, inspect the exact source/version and constrain chunk
metadata, dimensions, decoded output, internal buffers, encoder output and
concurrency independently; an actual resource/host-memory pilot must measure the
whole task. Cancellation inside upstream decode is not established. If strict
bounded behavior cannot be demonstrated, PNG profile stays unqualified; do not
silently weaken the cap or substitute source-only assurance. Alpha conversion,
16-bit/palette/interlacing and Go premultiplication compatibility remain required.

[crc32fast documentation](https://docs.rs/crc32fast/latest/crc32fast/) identifies
the IEEE CRC role. It is damage detection, not an authenticated MAC or ECC.
[BASE32 documentation](https://docs.rs/data-encoding/latest/data_encoding/constant.BASE32.html)
describes the padded RFC4648 alphabet. Legacy decoding compatibility still needs
independent vectors for padding/trailing bits and historical whitespace behavior.
[WASM Validator documentation](https://docs.rs/wasmparser/latest/wasmparser/struct.Validator.html)
distinguishes parsing from instruction/type validation. Select explicit module
features, reject component input for this profile, validate every payload and
bound module/section/candidate work; a valid header alone is insufficient.
Browser execution of an exported function remains a separate real pilot.

Indexed documentation currently displays older data-encoding2.11.0 and
wasmparser0.259.0 than the live registry candidates. Attempts to open pinned
version docs through the web tool were inaccessible initially. A subsequent
read-only direct HTTP request reached all four pinned official docs pages with
status200 and each requested version advertised. `pinned-api-review.json` binds
the decoded HTML UTF-8 digests, titles and role markers. This closes the document
access/version gap; it is not compilation, source audit, full API conformance or
runtime measurement. No handrolled crypto or executable foreign template is
proposed as a dependency workaround.

Pinned candidate pages:
[png0.18.1 Decoder](https://docs.rs/png/0.18.1/png/struct.Decoder.html),
[crc32fast1.5.4](https://docs.rs/crc32fast/1.5.4/crc32fast/),
[data-encoding2.11.1 BASE32](https://docs.rs/data-encoding/2.11.1/data_encoding/constant.BASE32.html),
[wasmparser0.263.0 Validator](https://docs.rs/wasmparser/0.263.0/wasmparser/struct.Validator.html).
The pinned PNG page retains the best-effort/caller-buffer caveat; the candidate
does not gain a hard whole-task memory guarantee from version confirmation.

## Advisory evidence and limits

The official [RustSec database tree](https://api.github.com/repos/RustSec/advisory-db/git/trees/main?recursive=1)
was inspected at commit309392b18f6b17022f82c701f300433126901068:2450 entries,
truncated=false,2401 under crates/. No paths matched the exact direct-crate
prefixes crates/png/, crates/crc32fast/, crates/data-encoding/ or crates/wasmparser/.
This is a dated direct-name database observation, not proof of safety or absence
of all vulnerabilities. It does not cover the unresolved transitive graph,
GitHub advisories outside RustSec, source audit or future disclosures. Recheck
the complete locked graph using the project's root-owned delivery advisory gates
before integration/release. Dependency publication/adoption is not authorized.
