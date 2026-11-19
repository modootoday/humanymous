---
id: plan:root/20261118133000
title: "IP reputation feed: datacenter input live, other categories shadow-only"
status: draft
created: 2026-11-18T13:30:00Z
updated: 2026-11-18T13:30:00Z
rel:
  depends_on: [sot:root/ip-reputation]
---

# IP reputation feed

## Goal

Owner decision, 2026-11-18: datacenter IPs get CHALLENGE. The frozen
`l5.ip.datacenter_asn` input (HR-11) is populated from public data. Tor-exit and
blocklist categories are counted and shown to operators, and never reach
scoring. No weight, threshold, predicate, verdict band or signal ID changes.

Branch `claude/ip-reputation-feed`, one pull request to `codex/rust`.

## Steps

1. **Pure core** (`network/intel/`). Done in this pull request.
   - Prefix set; snapshot format; source registry; per-format parsers;
     classification facade.
   - The PROXY v2 allowlist `Cidr` reuses the new `Prefix`.
2. **Store and serving path**. Done in this pull request.
   - Atomic snapshot swap, freshness from an injected clock, shadow counters.
   - Optional `ip_reputation` configuration, plus PROXY v2 client-address
     recovery on the HTTP/1.1 and TLS listeners.
   - Startup readiness, live management status, and a 60 s reload task.
3. **Updater**. Done in this pull request.
   - `humanymous ip-intel update|sources|inspect` in the operator CLI, behind
     the `ip-intel` feature.
   - Reads submodule checkouts without spawning git; fetches HTTPS; writes
     atomically.
4. **Sources as submodules**. Done in this pull request.
   - `third_party/ip-intel/x4bnet-datacenter` and
     `third_party/ip-intel/firehol-level1` are pinned gitlinks with
     `shallow = true`.
   - A test checks that `.gitmodules` equals the registry's git sources.
5. **Docs**. Done in this pull request: the how-to "Update the IP reputation
   feed".
6. **Docker product e2e** (later, owner or Root). Run the Docker topology with
   an edge that sends PROXY v2. Confirm that a datacenter source address gets
   CHALLENGE and a documentation-range residential address does not.

## Data-update process

- **Git sources.** The gitlink SHA is the data version.
  - An update is a reviewed pull request that only moves the gitlink: in the
    submodule run `git fetch --depth 1 origin <branch>` and check out the new
    commit, then `git add third_party/ip-intel/<name>`.
  - The PR body states the old and new SHA, the upstream licence at the new
    commit, and the prefix-count delta from `humanymous ip-intel inspect`.
  - No list data is copied into the repository.
- **HTTP sources.** Fetched by the operator at update time. Each source's URL
  and `fetched_at` are recorded in the snapshot. Nothing is committed.
- **Snapshot.** Operator state on the deployment host, never committed. The
  runtime reloads it when its length or modification time changes.

## Owner items

- Promote `tor_exit` or `blocklist` from shadow into scoring. This is a
  detection change: it needs measured shadow counts and accessibility review.
- FireHOL level1 for commercial deployments. It includes DShield data with
  non-commercial terms. The source is shadow-only; review it or deselect it.
- Edge deployment of PROXY protocol v2 toward the runtime listener. Without it,
  nothing is classified.
- Gitlink update cadence. Suggested: monthly, or when shadow counts drift.
- `Cargo.lock` gains `ureq` (already locked at 3.5.1) as an optional dependency
  of `humanymous-cli`.
- A Cloudflare zone-operator source is a possible future item. It is off by
  default and not implemented.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --locked -p humanymous-runtime --all-targets --all-features -- -D warnings`
- `cargo clippy --locked -p humanymous-cli --all-targets --all-features -- -D warnings`
- `cargo test --locked -p humanymous-runtime --all-features`
- `cargo test --locked -p humanymous-cli --all-features ip_intel`
- `spec check`
