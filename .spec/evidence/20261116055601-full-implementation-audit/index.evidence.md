---
id: evidence:root/20261116055601
title: Full implementation audit at 7a10291
status: active
created: 2026-11-16T05:56:01Z
updated: 2026-11-16T05:56:01Z
rel:
  depends_on: [sot:root/program-roadmap]
---

# Full implementation audit at 7a10291

## What was measured

A complete-implementation audit of `codex/rust` tip `7a10291` (committed
2026-11-10), run on 2026-11-16 in a single Linux cloud container as uid 0, with
Rust 1.99.1. Docker was installed there, but its daemon did not answer, so no
container run was possible. No CEF bundle, interactive AT, Windows or macOS
runner, and no `pwsh` were present.

| Probe | Observed |
| --- | --- |
| `cargo metadata --locked --offline --no-deps` | 29 workspace members |
| `cargo test --workspace --locked --no-fail-fast` | every target compiled; 112 test binaries; 1383 passed, 13 failed, 27 ignored; exit 101 |
| Failure: `humanymous-agent-maintenance --lib` (1) | lane catalog test pinned 25 lanes; `lanes.json`, `LANES.md` and both board scripts agree on 27. A real stale assertion, fixed in the same change as this receipt; the rerun passed 76/76 |
| Failure: `humanymous-cli --lib` (10) | `private non-root owned parent required`; a fail-closed refusal caused by running as root, environmental |
| Failure: `humanymous-agent-maintenance --test git_coord` (1) | `pwsh` absent (ENOENT), environmental |
| Failure: `humanymous-lab-runner` bin (1) | requires the private `.spec.local` Unlock A closer, which this checkout does not have. Depends on state outside the repository |
| Tracked Go sources | 336 files, 47761 lines |
| `.agents/retirement/legacy-go-owner-routing.json` | 314 baseline rows: 255 `pending`, 54 `ported_unit_parity_not_runtime`, 5 `partial_unit_parity_not_runtime`; 59 have a Rust mapping, 0 have exact runtime proof |
| `build/gate.Dockerfile`, `build/core.Dockerfile` | still build Go binaries |
| `spec check --no-local` before canonization | 69 errors (20 S2 slug ids, 42 L3, 2 I1, 2 S4, 1 T1, 1 L1, 1 L2) |

The last recorded product board (SoT program-roadmap, measured 2026-11-09) was
not re-measured because Docker could not run. It reads: product 0,
`pass_product_clean_room` 0, `full_product_qualified` false, BrowserReady
false, AUTH-HANDOFF and AT-HANDOFF absent, matrix 84 required / 47 clean-room
required / 0 product.

## How to read it

This is host and component evidence only. Per AGENTS.md, product e2e is
Docker-only and was not run, and native, browser and Cross-OS pilots were not run.
Passing unit tests do not qualify any milestone exit. The four environmental and
private-state failures are not counted as product defects or as passes. The full
test log was a session scratch file and is not retained.
