# Run the standalone detection engine (dev)

**Quadrant: How-to.** **Audience: Blue and Red developers who want to build and run the raw detection engine (`bin/server.exe` on `127.0.0.1:8443`) directly** — the engine-side equivalent of the Sentinel proxy Quickstart.

> **Note:** Defensive, local-only, self-target-only, educational. This page builds and runs *your own* detector on your own loopback so you can read its scoring output. It teaches nothing about evading third-party systems. Every number you observe is reference-measured on your machine, not a guarantee.

This page gets the detection engine compiled and serving on `127.0.0.1:8443`, shows you how to read a verdict with and without the Detection Observatory, and marks the boundary where the engine stops and the Sentinel proxy begins.

## When to use this instead of the Sentinel Quickstart

There are two binaries in this repo, and they do different jobs. The orientation map covers the full split — read [which piece am I using?](../explanation/which-piece-am-i-using.md) if you are unsure which one you want.

Use the **standalone detection engine** (this page) when you want the engine's per-request scoring across L1–L7 **without**:

- edge enforcement (the engine scores; it does not block a request at an edge),
- TLS termination or streaming bundle injection,
- the admin plane (no Audit Console, no bans, no dual-control),
- a tamper-evident audit log.

Use the **Sentinel reverse proxy** instead — via [Quickstart: monitor mode](../tutorials/quickstart-monitor-mode.md) — when you want those production behaviors wrapped around the same engine. Both binaries embed the same `internal/scoring` engine, so the verdicts are consistent; the difference is everything *around* the score.

## Build

The engine needs two artifacts: the browser detection bundle (`web/detector.wasm`) and the server (`bin/server.exe`).

Build the WASM bundle first:

```
GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/
```

> **Important:** `web/detector.wasm` is the browser-side detection bundle — the code that runs in the visitor's page to collect the L1–L4 client signals. The server serves it from `web/`, so if you skip this step the page loads without a detector and the client signals never arrive. Build it before (or alongside) the server.

Then build the server:

```
go build -o bin/server.exe ./cmd/server/
```

## Run

Start the engine on loopback, pointing `-web` at the directory that holds `detector.wasm`:

```
bin/server.exe -addr 127.0.0.1:8443 -web web
```

Flags: `-addr` (bind address), `-web` (static web root), `-rit` (request-integrity token, default `true`).

> **Warning:** The engine serves over TLS with a **self-signed development certificate**. Your browser and `curl` will flag it as untrusted — that is expected for a local dev cert. Trust it locally or pass your client's insecure/skip-verify flag for loopback testing only; never carry that habit to a real endpoint.

There is **no admin listener here** — the engine exposes `127.0.0.1:8443` only. The `:8445` admin plane and the `:8444` edge belong to the Sentinel proxy, not to this binary.

## The `HMN_PLAYGROUND` gate

The Detection Observatory is a dev-gated page that runs *on this engine*. Whether it exists is controlled by one environment variable.

- **`HMN_PLAYGROUND` unset** — plain engine. The Observatory routes are not registered, the hub is nil, and there is **zero added surface and zero request-path cost**. This is the default and what you want for straight scoring.
- **`HMN_PLAYGROUND=1`** — the Observatory routes appear (`/playground`, `/playground/events`, `/playground/explain/{id}`, and the rest), and the **loopback fail-closed rule applies**: the server refuses to start on a non-loopback `-addr`, and every request's `Host` is re-checked for loopback.

To understand what those routes do and why they are safe to expose only locally, see [how the Detection Observatory is built](../explanation/observatory-architecture.md); to actually drive it, see the [Detection Observatory how-to](./detection-observatory.md).

## See a verdict without the Observatory

You do not need the Observatory to read a score — it is a viewer, not the scorer.

**Load a page in a browser.** Point a browser at `https://127.0.0.1:8443/` (accepting the dev cert). The `detector.wasm` bundle collects the client signals and POSTs a session report to `/api/collect`; the response is the scored verdict as JSON, containing `verdict` (`ALLOW` | `CHALLENGE` | `DENY`), `riskScore` (0–100), `hardRuleFired`, and `topContributors`.

**Or run a Red profile against it.** The Red runner drives one profile and prints the verdict:

```
node test/redteam/run-one.mjs human https://127.0.0.1:8443
```

`run-one.mjs` prints a minimal JSON line — `{label, verdict, riskScore, hardRuleFired}` — and intentionally drops `topContributors`. For the full per-layer trace, use the Observatory's `/playground/explain/{id}` or the e2e `results.json`; see the [self-validation how-to](./self-validation-red-team.md).

Post a minimal client report directly and read the verdict (the `-k` accepts the self-signed dev cert):

```
curl -sk -X POST https://127.0.0.1:8443/api/collect \
  -H "Content-Type: application/json" \
  -d '{"userAgent":"Mozilla/5.0 Chrome/126","signals":[{"id":"l1.navigator.webdriver","layer":"L1","verdict":"BOT","weight":40,"score":40,"confidence":0.95,"collected":"js"}]}'
```

The response is the ScoreResult: `{"sessionId":…,"riskScore":…,"verdict":…,"hardRuleFired":…,"topContributors":[…],"policyVersion":…}`. The `signals` array is the client-collected L1–L4 portion; the server merges its own L5/L6 network signals before scoring.

The score itself comes from `internal/scoring`: each signal contributes `weight x severity x confidence`, the combiner deduplicates per group, caps each layer, and folds the layers with a noisy-OR into a 0–100 risk score, which policy `1.0.0` maps to `ALLOW` / `CHALLENGE` / `DENY` (challenge at 30, deny at 70). Hard rules HR-1..HR-21 can promote a verdict ahead of the score. For the full walk-through of how a request becomes a verdict, read [how Sentinel sees a request](../concepts/how-sentinel-sees-a-request.md) and [detection engine internals](../explanation/detection-engine-internals.md).

## The boundary: what this engine does not do

This binary **scores**. It does not enforce at an edge, and it does not write the tamper-evident audit log. Those are the Sentinel reverse proxy's job — the proxy wraps this same engine and adds TLS termination, edge enforcement, the audit log, the Audit Console, bans, and dual-control.

If you need any of that, do not try to bolt it onto the standalone engine — run the proxy instead, starting from [Quickstart: monitor mode](../tutorials/quickstart-monitor-mode.md). A promoted Sentinel build never exposes the Observatory, and the Observatory (`:8443`) is never the Audit Console (`:8445`).

## Related

- [Which piece am I using?](../explanation/which-piece-am-i-using.md) — the two-engine orientation map.
- [Quickstart: monitor mode](../tutorials/quickstart-monitor-mode.md) — the Sentinel proxy equivalent of this page.
- [Detection Observatory how-to](./detection-observatory.md) — drive the dev-gated viewer on this engine.
- [Detection engine internals](../explanation/detection-engine-internals.md) — where the score comes from.
