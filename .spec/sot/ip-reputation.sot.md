---
id: sot:root/ip-reputation
title: Operator IP reputation feed
status: draft
created: 2026-11-18T13:30:00Z
updated: 2026-11-18T16:30:00Z
verify: cargo test --locked -p humanymous-runtime --all-features network::intel && cargo test --locked -p humanymous-runtime --all-features --test network_ip_intel && cargo test --locked -p humanymous-cli --features ip-intel ip_intel
rel:
  governs: [crates/sdk/humanymous-runtime/src/network/intel/**, crates/sdk/humanymous-runtime/src/gateway/configuration/ip_reputation.rs, crates/sdk/humanymous-runtime/tests/network/ip_intel.rs, crates/sdk/humanymous-runtime/tests/fixtures/ip_intel/**, crates/cli/src/ip_intel/**, third_party/ip-intel/**]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application, sot:root/cli-installation]
---

# Operator IP reputation feed

Document version 0.3.0.

## Decision

Owner decision, 2026-11-18: datacenter IPs must get CHALLENGE. The product
exists to raise the cost of automation, and datacenters host or prepare most of
it.

The frozen engine already does this. `l5.ip.datacenter_asn` fires hard rule
HR-11 (CHALLENGE). Until this SoT, the serving path never set the
`is_datacenter_ip` observation input, so HR-11 could not fire in production.
The Go oracle's `SetDatacenterCIDRs` was only called from tests and failed open
on an empty list.

This SoT populates that **existing** input from public data. It adds no signal,
weight, threshold, hard-rule predicate, verdict band or signal ID. The detection
freeze is unchanged.

## Categories

| Category | Effect | Signal |
|---|---|---|
| `datacenter` | Sets `is_datacenter_ip` | Existing `l5.ip.datacenter_asn` (HR-11) |
| `tor_exit` | **Shadow-only**: counted, shown to operators | None |
| `blocklist` | **Shadow-only**: counted, shown to operators | None |

- The serving path never sets `is_tor_exit` or `is_proxy` from this feed, even
  though the frozen report has inputs for them. Doing so would change verdicts
  outside the owner decision.
- `IntelCategory::feeds_scoring` is true only for `datacenter`. It is code, not
  configuration: an operator cannot promote a category by editing a file.

### Owner item: promotion

Promoting `tor_exit` or `blocklist` into scoring needs a separate owner
decision. That decision must cover false-positive measurement on shadow counts,
accessibility review, and the blocked-user help path. Until then both stay
shadow-only.

## Sources

Only free, public sources. None needs a paid service or an owner-owned account.
Two acquisition paths exist.

- **Pinned git submodule.** The source lives in an external git repository.
  - It is a submodule under `third_party/ip-intel/<registry name>`, pinned by
    commit (a gitlink). `shallow = true` is set in `.gitmodules`.
  - The gitlink SHA is the source of truth for which data version is used.
  - Updating data means a reviewed PR that moves the gitlink. That PR's diff
    shows the old and new SHA.
  - The updater reads the checkout. It records the checked-out commit in the
    snapshot, with a `checkout` record: the parent repository's gitlink
    (`git ls-tree HEAD <path>`), whether the tree is clean, and whether the
    operator overrode a mismatch.
  - A checkout whose HEAD differs from the gitlink, that has local changes, or
    whose gitlink cannot be read is refused as `invalid`
    (`submodule_head_mismatch`, `submodule_dirty`,
    `submodule_gitlink_unreadable`) unless the operator passes
    `--allow-unpinned-checkout`; the snapshot then records
    `unpinned_override: true`, and the runtime status reports
    `source_unpinned:<name>` in `degraded_reasons` while that snapshot is
    active.
  - No list data is copied into this repository.
- **Operator fetch.** The source is only an HTTP endpoint.
  - The updater fetches it at update time over HTTPS.
  - It records the URL and `fetched_at` in the snapshot.
  - Nothing is vendored.

A test asserts that the `.gitmodules` entries under `third_party/ip-intel/` are
exactly the registry's git sources: same set, same paths, same URLs.

| Registry name | Category | Path | Terms (checked 2026-11-18) |
|---|---|---|---|
| `aws-ip-ranges` | datacenter | HTTP `ip-ranges.amazonaws.com/ip-ranges.json` | Public AWS endpoint; no redistribution grant. Operator fetch. |
| `gcp-cloud` | datacenter | HTTP `www.gstatic.com/ipranges/cloud.json` | Public Google endpoint; no redistribution grant. Operator fetch. |
| `azure-service-tags` | datacenter | Rotating Download Center URL | Microsoft Download Center terms. The operator passes the current URL from the discovery page (`--url`). |
| `oracle-cloud` | datacenter | HTTP `docs.oracle.com/.../public_ip_ranges.json` | Public Oracle endpoint. Operator fetch. |
| `digitalocean-geofeed` | datacenter | HTTP `digitalocean.com/geo/google.csv` (RFC 8805) | Self-published geofeed. Operator fetch. |
| `linode-geofeed` | datacenter | HTTP `geoip.linode.com` (RFC 8805) | Self-published geofeed. Operator fetch. |
| `x4bnet-datacenter` | datacenter | Submodule `X4BNet/lists_vpn` at `c60005a45e8b93dbdaa5d5d750de81a76a9d6a0f`, `output/datacenter/ipv{4,6}.txt` | MIT for the generated lists. The upstream IPLocate ASN data is CC BY-SA 4.0: attribute it, and share alike if the snapshot is redistributed. |
| `tor-bulk-exit` | tor_exit | HTTP `check.torproject.org/torbulkexitlist` | Tor Project public list. Operator fetch. |
| `spamhaus-drop` | blocklist | HTTP `www.spamhaus.org/drop/drop_v{4,6}.json` | Spamhaus DROP terms: free of charge; no IP licence; no use of the Spamhaus name in marketing; revocable. Operator fetch only. |
| `firehol-level1` | blocklist | Submodule `firehol/blocklist-ipsets` at `0df25fdb2d7184ff3da4f7db086f4cb39ba415fa`, `firehol_level1.netset` | No repository licence. Each list keeps its upstream terms (it includes DShield, Feodo, fullbogons and Spamhaus DROP). The gitlink is a pointer, not a copy. See the owner item below. |

### Considered and not included

- `jhassine/server-ip-addresses`: the repository has no licence. It also only
  re-publishes AWS, Azure, GCP and Cloudflare, which are already covered.
- `client9/ipcat`: MIT, but last updated 2023-02, so too stale.
- Hetzner and OVH: neither publishes a machine-readable range list. Their ASNs
  are covered through the X4BNet ASN-derived list.

### Owner items: licences

- **FireHOL level1 for commercial operators.** FireHOL level1 includes DShield
  data, whose terms are non-commercial. The source is shadow-only and is only
  read locally. Still, commercial operators should review the upstream terms,
  or skip the source (`--source` selects sources).
- **Spamhaus DROP.** The terms allow free use and revocation. The project must
  not use the Spamhaus name in marketing.

### Future option: Cloudflare zone-operator data

A Cloudflare zone operator could feed its own edge's threat or bot fields. That
is optional and off by default, and is not implemented. It depends on an
owner-owned account, which this SoT excludes.

## Snapshot format

The updater writes one UTF-8 JSON document, `humanymous-ip-intel/1`:

- `format`: exactly `humanymous-ip-intel/1`. Unknown fields are refused.
- `generated_at`: Unix seconds. Staleness is measured from this value.
- `sources[]`: sorted by name. Each source has:
  - `name`: `[a-z0-9-]{1,64}`, unique;
  - `category`;
  - `licence`;
  - `origin`: one of `{git:{path, commit, checkout}}`,
    `{http:{url, fetched_at}}` or `{file:{read_at}}`. `checkout` is
    `{gitlink, clean, unpinned_override}` and is present exactly when
    `commit` is;
  - `status`: `ok`, `missing` or `invalid`;
  - `detail`: a static reason, present unless `ok`;
  - `prefixes[]`: canonical, sorted CIDRs.

Validation rules:

- Sources are strictly sorted by name. Each source's prefixes are canonical
  (no host bits), strictly ascending and therefore unique.
- A source whose name is in the registry must carry the registered category.
  Editing a file cannot promote `tor-bulk-exit` to `datacenter`.
- An `ok` source needs prefixes. `missing` and `invalid` sources have none.
- A git source with `ok` status needs a 40- or 64-hex commit. Its `checkout`
  must either match (gitlink equals commit, tree clean, no override) or record
  `unpinned_override: true`.
- `generated_at` may be at most 300 s in the future.
- `fetched_at` and `read_at` may not be later than `generated_at`.

Breadth guard for scoring categories (`datacenter`), checked per source by
the updater and per category (union of all sources) on encode and on every
runtime load:

- no prefix shorter than IPv4 /8 or IPv6 /19 (`snapshot_prefix_too_broad`);
- at most 2^30 IPv4 addresses (a quarter of the family) and 2^112 IPv6
  addresses (one /16) after merging (`snapshot_coverage_cap`).

Measured 2026-11-18, the numbers these values are chosen against:

| Data | Shortest IPv4 | Shortest IPv6 | IPv4 coverage | IPv6 coverage |
|---|---|---|---|---|
| Pinned X4BNet `c60005a4…` (`output/datacenter`) | /8 | /20 | 408,109,015 (2^28.60) | 2^109.76 |
| AWS, Google Cloud, Oracle, DigitalOcean, Linode endpoints, each | /11 or longer | /32 or longer | at most 2^26.61 | at most 2^101.17 |
| Union of all of the above | /8 | /20 | 2^28.65 | 2^109.76 |

Azure Service Tags were not measured: the URL rotates and is operator-supplied.

- IPv6 floor /19: one bit of headroom under the shortest measured prefix
  (X4BNet /20). A /16-sized carrier block is refused by itself.
- IPv6 cap 2^112: about 2.2 bits (4.7x) above the measured 2^109.76.
  `2600::/12` (Comcast, Verizon, T-Mobile and other North American
  residential and mobile carriers) is refused either way: split into /16s
  every piece is under the floor; split into /20s the union is 2^116, over
  the cap. The earlier values (/16 floor, 2^116 cap) let sixteen /16s covering
  `2600::/12` pass.
- IPv4 floor /8 and cap 2^30 are unchanged: the measured union is 2^28.65.
- Regression coverage: a unit test refuses `2600::/12` split both ways; a
  fixture with X4BNet's measured shortest prefixes and exact coverage passes;
  and an ignored test reads the real pinned list when the submodule is
  initialized and asserts it still has those exact numbers. Run without the
  data, that ignored test fails with the reason; it is never reported as a
  pass.

Shadow-only categories are not bounded by breadth: they never reach scoring,
and FireHOL level1 legitimately carries bogon blocks such as `224.0.0.0/3`.

When every scoring source fits alone but their union exceeds the cap, the
updater fails closed per source rather than aborting the whole update. It
drops the `ok` source with the largest coverage in the overflowing family
(ties: the lowest name), marks it `invalid` with the reason
`category_union_coverage_cap`, and repeats until the union fits. Each drop is
recorded in the snapshot and the report. The remaining sources are written, and
a union above the cap is never written.

A refused snapshot never activates scoring: on first load the store stays
`missing`, on reload it keeps the previous snapshot, and either way the status
reports the code in `last_reload_error`.

Bounds:

- 64 MiB per file;
- 64 sources;
- 2,000,000 prefixes in total;
- 512-byte text fields;
- 32 MiB and 1,000,000 prefixes per upstream source.

The encoder and parser share one validator. A file the updater can write is
exactly a file the runtime accepts.

## Runtime behaviour

The layout follows `sot:root/rust-migration` (one owner per responsibility; pure
code kept apart from I/O):

| Path | Responsibility |
|---|---|
| `network/intel/mod.rs` | Facade: `IntelCategory`, `Classification`, and `IntelReader`, the one read contract the serving path uses |
| `prefix_set/` | Pure IPv4/IPv6 prefix set; sorted merged ranges with binary search. Its `Prefix` is the network context's only CIDR parser; the PROXY v2 allowlist reuses it. |
| `snapshot/` | Pure format: parse, validate, encode |
| `source/` | Pure registry as data, plus `normalize(format, bytes)`. Parsers are grouped by format: `aws_json`, `gcp_json`, `azure_tags`, `oracle_json`, `geofeed`, `netset`, `spamhaus_jsonl`. |
| `store/` | I/O boundary: bounded load, atomic `RwLock<Arc<_>>` swap, staleness from an injected clock, ops status |
| `shadow/` | Shadow-only counters |

Configuration lives in the runtime's existing configuration surface, as the
optional `ip_reputation` object:

- `snapshot`;
- `client_address`: `proxy_v2`;
- `max_age_s`: default 7 days;
- `expire_after_s`: default 30 days;
- `categories`: default all;
- `proxy_header_timeout_ms`: default 1000.

When the object is absent, behaviour is unchanged.

### Trusted client address

The runtime listener accepts loopback connections only. The original client
address therefore comes from the trusted edge, through a **PROXY protocol v2**
header on every accepted connection. The runtime reuses the existing
`network::proxy` parser.

- A connection without a valid header is dropped. A misconfigured edge fails
  closed and is not classified.
- The header wait ends at the earlier of the request deadline and
  `proxy_header_timeout_ms` (default 1000 ms, bounds 10..=30000), and a
  shutdown request ends it at once, so a silent peer cannot hold the
  serialized accept loop or delay shutdown.
- LOCAL or UNSPEC headers mean the request is unattributed.
- Only the commands LOCAL (0x0) and PROXY (0x1) are accepted. A reserved
  command (0x2 to 0xF) is refused and the connection dropped; it is never
  treated as PROXY.
- HTTP forwarding headers are never trusted. Native request integrity signs
  every header, and a client controls any header the edge forgets to strip.
- Private, loopback, link-local and unspecified addresses are never classified.
  This matches the Go oracle.

### Freshness and failure semantics

Traffic fails open, but the state is never silent.

| State | Classification | Visibility |
|---|---|---|
| `missing` (no snapshot accepted yet) | Nothing | Startup readiness `ip_reputation.startup_state`; live management status |
| `fresh` (age ≤ `max_age_s`) | All enabled categories | Status |
| `stale` (age ≤ `expire_after_s`) | Still classifies; counted in `lookups.stale` | Status `state: stale`, `age_s` |
| `expired` | Nothing; counted in `lookups.unavailable` | Status `state: expired`, `datacenter_scoring_active: false` |

Further rules:

- A refused reload keeps the previous snapshot. The status reports
  `last_reload_error`, `reloads_failed` and the snapshot SHA-256.
- The status reports `degraded` and `degraded_reasons` whenever scoring
  coverage is incomplete: `snapshot_missing|stale|expired`, `reload_refused`,
  and per registry or snapshot datacenter source `source_absent:<name>`,
  `source_missing:<name>`, `source_invalid:<name>`, `source_unpinned:<name>`
  (a git checkout accepted with `--allow-unpinned-checkout`) or
  `source_stale:<name>` (HTTP fetch or file read older than `max_age_s`). Old data for a failed
  source is never carried forward; its coverage is absent and named here.
  Readiness repeats the startup reasons as `startup_degraded_reasons`.
- A missing or refused feed never denies all traffic.
- It is also never reported as clean: `datacenter_scoring_active` is false, and
  `missing` or `invalid` sources stay listed with their reason.
- The updater refuses to write a snapshot when no source succeeded. When any
  selected source is missing or invalid, it writes the snapshot and exits
  non-zero.
- An uninitialized submodule is reported as `missing` with the reason
  `submodule_not_initialized`, not as an empty list.
- The runtime checks the snapshot file's length and modification time every 60
  seconds, and parses on the blocking pool.
- Live status is served by the management observer route
  `POST /management/v0.1/ip-reputation`. It carries the same authentication and
  scope checks as `/status`.

## Updater

The existing operator CLI is the composition root:

```
humanymous ip-intel update|sources|inspect
```

It is behind the `ip-intel` Cargo feature, so the default offline provisioning
binary has no HTTP client. `update` only wires these steps:

1. Read submodule checkouts. The commit comes from the checkout's git metadata.
   The pin check runs `git ls-tree HEAD -- <path>` in the repository and
   `git status --porcelain` in the checkout.
2. Read operator files (`--input NAME=PATH`).
3. Fetch HTTPS bodies, with a 60 s timeout and the per-source byte bound.
4. Call `source::normalize` and `snapshot::check_breadth` per source, drop
   sources until each scoring category's union fits the coverage cap (see
   the breadth guard), then `snapshot::encode`.
5. Write a private sibling file with `create_new`, sync it and rename it over
   the output.

Arguments are refused, never ignored: a `--source` named twice, an unknown
source name in `--source`, `--input` or `--url`, and `--url` for a source
whose endpoint is fixed or pinned (anything but `azure-service-tags`) all
fail at parse time. `update` repeats the same check before acquiring anything.

`--offline` never fetches. Unit tests inject a fake fetcher, and builds and
tests need no network or submodule access.

## Limits (T4 ceiling)

- Residential proxies, mobile carriers and commercial VPNs on consumer ranges
  are not covered. A coherent real browser on a residential exit stays
  invisible to this feed. That is the declared T4 ceiling.
- Range lists lag reassignment. Stale data can challenge a human on a
  reassigned range. `expire_after_s` bounds how long that can last.
- HR-11 challenges and never denies. A human on a cloud desktop or corporate
  egress inside a cloud range gets a challenge and must be able to pass it.
  Pass and CHALLENGE never depend on motor richness or speed.
- Without PROXY v2 from the edge, nothing is classified. The authority adapter
  path (`ApplicationHttp`) has no trusted client address and stays
  unclassified.

## Privacy

- The status and the snapshot contain prefixes and counters only. They contain
  no client addresses, sessions or request data.
- The classification is not written to the audit record beyond the existing
  verdict and triggered rule.
- Shadow counters are process-lifetime aggregates with no per-client keys.
