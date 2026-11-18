---
layout: doc
title: Update the IP reputation feed
permalink: /how-to/update-ip-reputation-feed.html
reviewed: 2026-11-18
---
# Update the IP reputation feed

**Diátaxis quadrant:** How-to. **Audience:** operators evaluating the runtime
gateway.

## Limits first

- The feed covers published cloud and hosting ranges only. It does not detect
  residential proxies, mobile carriers or commercial VPNs on consumer ranges. A
  real browser on a residential exit is not visible to it.
- Only the datacenter category affects decisions. A datacenter client gets a
  CHALLENGE through the existing datacenter rule, and is never denied.
- Tor-exit and blocklist matches are **shadow-only**. They are counted in the
  status and change no verdict.
- The runtime classifies a client only when the edge sends a PROXY protocol v2
  header on every connection. HTTP forwarding headers are never trusted.
- A missing, refused or expired snapshot never blocks traffic. It shows up in
  readiness and in the live status instead.
- Upstream terms differ per source. Run `humanymous ip-intel sources` to see
  each source's licence note. Skip any source your use does not permit.

## 1. Build the updater

The updater is an optional feature of the operator CLI:

```
cargo build --locked --release -p humanymous-cli --features ip-intel
```

## 2. Initialize the pinned git sources

Two sources are git submodules under `third_party/ip-intel/`. Their pinned
commit is the data version:

```
git submodule update --init --depth 1 third_party/ip-intel/x4bnet-datacenter
git submodule update --init --depth 1 third_party/ip-intel/firehol-level1
```

If a submodule is not initialized, the updater still writes the snapshot. That
source is listed as `missing` with the reason `submodule_not_initialized`, and
the command exits non-zero.

## 3. Write a snapshot

Run the updater from the repository root:

```
humanymous ip-intel update --output /var/lib/humanymous/ip-intel.json \
  --url azure-service-tags=<current ServiceTags_Public JSON URL>
```

- **Azure.** The Azure download URL changes weekly. Copy the current one from
  Microsoft's Service Tags download page, or leave it out; Azure is then
  reported as `missing`.
- **Choosing sources.** `--source NAME` (repeatable) selects sources.
- **Local files.** `--input NAME=PATH` reads a file you already downloaded.
- **No network.** `--offline` reads only submodules and `--input` files.
- **Exit status.** The command writes nothing when no source succeeds. When
  some sources fail, it writes the snapshot, prints the report and exits
  non-zero.

Check the result:

```
humanymous ip-intel inspect /var/lib/humanymous/ip-intel.json
```

## 4. Point the gateway at it

Add the object to the gateway configuration JSON:

```json
"ip_reputation": {
  "snapshot": "/var/lib/humanymous/ip-intel.json",
  "client_address": "proxy_v2"
}
```

- **Edge.** Configure your edge to send PROXY protocol v2 to the gateway
  listener. Connections without a valid header are dropped.
- **Freshness.** Optional fields: `max_age_s` (default 7 days, after which the
  snapshot is reported stale) and `expire_after_s` (default 30 days, after
  which it classifies nothing).
- **Reloading.** The gateway checks the file every 60 seconds. It keeps the
  previous snapshot when a new one is refused.

## 5. Watch the status

- **Startup.** Readiness shows the `ip_reputation` state at startup.
- **Live.** The management observer route `POST /management/v0.1/ip-reputation`
  returns:
  - freshness and age;
  - per-source status;
  - reload errors;
  - lookup counters;
  - shadow hit counts.
- **Privacy.** The status contains no client addresses.

## Updating the pinned sources

A data update for a git source is a reviewed pull request that only moves the
gitlink:

```
cd third_party/ip-intel/<name>
git fetch --depth 1 origin <branch>
git checkout FETCH_HEAD
cd -
git add third_party/ip-intel/<name>
```

In the pull request, state the old and new commit, the upstream licence at the
new commit, and the prefix count reported by `inspect`. Never copy list files
into the repository.
