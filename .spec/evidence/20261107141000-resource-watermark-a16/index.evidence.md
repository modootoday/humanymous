---
id: evidence:root/20261107141000
title: Watermark A16 native pure component verification
status: active
created: 2026-11-07T14:10:00Z
updated: 2026-11-07T14:12:07Z
rel:
  depends_on: [sot:root/resource-watermark, plan:root/20261107135500]
files:
  - {path: result.json, sha256: f1d720c2439fa48eebe076214b9d82e6ef69d711d8fe447adab036bf8a0da2e6, bytes: 2581, store: repo}
---

# A16 component verification

Root explicitly assigned this owner after the four performance B1 builds ended.
Exact source9b064dbd8942d4fad7f1623da97b6b64c0177a7f was extracted from a Git
archive into an isolated source directory. Archive SHA256 is
055b8c86318d4fdb030d87378497c90028deb458088fc8eada253a8fc4087015,
69294080 bytes. No code or package/lock changes were made for this run.

Native Windows x86_64-pc-windows-msvc, Rust1.99.1, jobs1/incremental0,
offline/locked, root-reserved existing target validation-audit. Initial admission
was82.1889% committed and30.7731GiB free; all gate admissions were rechecked.
No compiler was observed at initial admission. The existing target is reused,
not a cold-cache benchmark. This is not a performance comparison.

## Actual terminal results

Controller38464 / tool session65950 ended with exit0. All four commands ended0:

- cargo fmt -p humanymous-watermark -- --check
- cargo check --locked --offline -p humanymous-watermark --all-targets --jobs 1
- cargo clippy --locked --offline -p humanymous-watermark --all-targets --jobs 1 -- -D warnings
- cargo test --locked --offline -p humanymous-watermark --all-targets --jobs 1 -- --test-threads=1

Actual footer:16 passed;0 failed;0 ignored;0 measured;0 filtered out. The six
payload and ten ledger cases ran, including ID exhaustion, clock regression,
capacity/TTL/GC, reservation unwind/poison refusal, concurrency, NUL ambiguity,
suffix compatibility, literal HMAC input and the RFC4231 primitive vector.
This is the first execution of this source; there was no preceding test failure
or correction concealed by this receipt. The unit executable was
humanymous_watermark-620887680b1a5a84.exe. No codec, HTTP, browser, actual resource
pilot or Docker product scenario was executed.

## Source and input closure

The complete extracted archive, conservative selected dependency source closure,
pinned toolchain bin files, controller binary and present Cargo configuration
paths were enumerated before and after:3531 files each,0 difference records.
Breakdown:2539 archive files,970 registry source files across20 registry packages,
21 pinned toolchain bin files,1 controller binary. Workspace-wide feature
unification makes the selected metadata graph conservative; not every listed
dependency is compiled for native Windows. The registry manifests and all files
under those selected source roots are included. No ancestor or user Cargo config
file was found at the recorded paths; no CARGO_HOME override was set.

This does not snapshot every Windows SDK/linker/system DLL or host OS file and
does not establish complete native toolchain immutability or a clean shared tree.
Environment/configuration identities, admissions, tool versions and raw logs are
retained privately in `.spec.local/root/evidence/20261107141000-resource-watermark-a16/`.
The public result.json records exact commands, times and compared-file counts.
Counts of files and difference records are separate quantities. A config/tool
path outside the enumerated closure is not retroactively declared unchanged.

Target validation-audit was returned to root after terminal result/footer
inspection; no owned compiler/test process remained. The stored persistent goal
status was previously blocked and its API has no resume action; this receipt
records authorized resumed work without falsely marking the overall goal complete.

## Remaining full-goal gates

Initial payload/carrier and bounded ledger component verification passed. This
does not satisfy the original fourteen-file migration. CRC frame, MIME dispatch,
text/binary/PNG/LSB/WASM codecs, inject/extract and authenticated trace integration,
all original format tests and real resource pilots remain incomplete. V3–V5 and
root's full workspace/product acceptance remain required. No scoring, permanent
ban, human-proof or feature-loss-free full parity claim follows from these units.
