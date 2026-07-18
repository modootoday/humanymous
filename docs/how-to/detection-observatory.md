# Watch detection happen live: the Detection Observatory

**Diátaxis quadrant:** How-to. **Audience:** Red/Blue developers who want to *see* the detection engine reach a verdict, layer by layer, while testing on their own machine.

The Detection Observatory is a real-time, local-only web page served by the detection engine. You fire a bundled attack profile — or just browse — at your own engine and watch the L1→L7 pipeline light up: which signals fired, how the risk score built to its band, and which hard rule (if any) overrode it. It turns the terminal-only test run into something you can watch and learn from.

> **Tip:** Want to see what it looks like first? The **[guided tour](observatory-tour.md)** is an annotated screenshot walkthrough.

> **Note:** The Observatory runs on the standalone detection engine (`bin/server.exe` on `127.0.0.1:8443`) — **not** the Gate proxy (`bin/gate.exe` on `:8444`/`:8445`) you build in the Quickstart. It is a separate binary for watching the L1→L7 scoring in isolation, and it is not the Ledger. If those two binaries are new to you, read [Which piece am I using?](../explanation/which-piece-am-i-using.md) first. For how the live feed and safety model work, see [Inside the Detection Observatory](../explanation/observatory-architecture.md).

> **Warning:** The Observatory is a **development-only, loopback-only** surface. It spawns local test processes and streams raw detection telemetry, so it is disabled by default and refuses to start on a non-loopback address. Never enable it on a production or internet-reachable deployment. A promoted Gate build never exposes it.

> **Note:** This validates *your own* engine on `127.0.0.1`. There is no target field — the Red side can only fire the bundled catalog at your local engine. It is not a tool for probing systems you do not operate, and it contains no evasion tuning.

---

## Step 1 — Build the detection engine

From the repository root, build the browser detection bundle and the server:

```
GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/
go build -o bin/server.exe ./cmd/server/
```

## Step 2 — Start the engine with the Observatory enabled

The Observatory is gated behind an environment variable and a **loopback** listen address. Set `HMN_PLAYGROUND=1` and bind to `127.0.0.1`:

```
HMN_PLAYGROUND=1 bin/server.exe -addr 127.0.0.1:8443 -web web
```

> **Important:** With `HMN_PLAYGROUND=1`, the server **refuses to start** on a non-loopback address (for example `-addr :8443`, which binds all interfaces). This is deliberate: the launch endpoint spawns local processes, so the surface must never be remotely reachable. If you see `requires a loopback -addr`, change `-addr` to `127.0.0.1:8443` or `localhost:8443`.

When the flag is unset, none of the `/playground` routes exist and the engine serves exactly as it does normally — there is zero added surface and zero added cost on the request path.

## Step 3 — Open the page

Open the Observatory in a browser:

```
https://127.0.0.1:8443/playground
```

Expect a certificate warning — the development certificate is self-signed. Accept it to proceed. The status indicator reads **live · SSE connected** once the page has attached to the live feed.

## Step 4 — Watch your own session

The simplest thing to observe is your own browser. As the page loads it is scored like any other visitor, and the result streams onto the pipeline. You will see:

- **The risk gauge and verdict banner** — the 0–100 score, the band it falls in (`0–29` ALLOW, `30–69` CHALLENGE, `70–100` DENY), and any hard rule that fired.
- **The L1→L7 waterfall** — each firing signal as a chip in its layer lane, colored and shape-coded by its verdict (■ BOT, ◆ SUSPICIOUS, ● OK). Server-observed network signals (L5/L6) carry an inset tint.
- **The cross-check board (L6)** — each identity-consistency check as consistent or inconsistent.

## Step 5 — Fire a bundled profile at your engine

The **Red launcher** on the left lists the bundled catalog, grouped (baseline / browser-drivers / stealth / non-browser / anti-bypass / frontier), each card showing its documented tell and the hard rule you expect it to trip.

1. Click a profile (for example `selenium` → HR-1, or `direct_cdp` → HR-9).
2. Optionally set **runs** (1–5).
3. Click **Launch (local)**.

The launcher requests a single-use nonce, then fires exactly that one profile at `127.0.0.1:8443` — the target is fixed in the server; there is no host field. The run drives a real request through the engine, and the scored session streams onto the pipeline. The **Abort** button cancels an in-flight run.

> **Note:** Browser-driver profiles (`selenium`, `puppeteer`, `playwright`, …) need a local browser (Playwright / Edge) installed. If a profile's dependency is absent, the run reports a skip reason in the live event log rather than failing silently. The `human`, `http_client`, and `tls_*` profiles run without a browser.

Before each launch the engine's stateful detectors (rate limiter, cross-session correlation, traffic log) are reset, and launches are serialized — so a flood run cannot poison the baseline of your next run, and a second launch while one is in flight returns "a launch is already in flight."

## Step 6 — Read "why this verdict"

Under the pipeline, the **Why this verdict** panel shows the server's own decision trace for the current session — not a re-implementation:

- **The hard-rule ladder** — every engine rule (HR-1 through HR-21) evaluated in **first-match precedence order** (not numeric order — for example HR-13 is checked before HR-10), with the matched ones highlighted and the **winner** marked. The winner overrides the score band regardless of the number, and each rule shows a plain-language reason. (HR-22..HR-30 are Gate-plane rules and never appear here.)
- **The per-layer decomposition** — each layer's combined probability, and where the per-layer cap clamped a saturated layer (for example `L6 84% → cap 60%`).
- **Dedup drops** — where two tells from the same root cause were de-duplicated so they weren't double-counted.

This is the honest explanation of how the score and verdict were reached.

## Step 7 — Network-layer attacks

Some attacks never complete a normal scored request — an HTTP/2 Rapid Reset (`rapid_reset`) resets its streams by design. These surface on the live event log as a **network.abuse** event, so the pipeline is not blank for exactly the connection-level attacks. Fire `rapid_reset` and watch for it.

---

## Endpoints (for scripting)

All routes are loopback-only and exist only when `HMN_PLAYGROUND=1`.

| Method & path | Purpose |
|---|---|
| `GET /playground` | The Observatory page. |
| `GET /playground/meta` | Policy constants (thresholds, layer cap) and the hard-rule range. |
| `GET /playground/events` | The live SSE feed (`session.scored`, `attack.*`, `network.abuse`). Honors `Last-Event-ID`. |
| `GET /playground/explain/{id}` | The decision trace (per-layer decomposition, ordered hard-rule evaluation) for a stored session. |
| `GET /playground/nonce` | A single-use launch nonce. |
| `POST /playground/launch` | Fire one bundled profile: `{"profileId":"selenium.mjs","runs":1,"nonce":"…"}`. Any `host`/`url`/`upstream` key is rejected. |
| `POST /playground/abort` | Cancel the in-flight run. |

Example — fire one profile from the shell:

```
NONCE=$(curl -sk https://127.0.0.1:8443/playground/nonce | grep -o '"nonce":"[^"]*"' | cut -d'"' -f4)
curl -sk -X POST https://127.0.0.1:8443/playground/launch \
  -H "Content-Type: application/json" \
  -d "{\"profileId\":\"http_client.mjs\",\"runs\":1,\"nonce\":\"$NONCE\"}"
```

---

## What it is — and is not

- It **shows you, live, how your own engine decides.** The score decomposition and hard-rule ladder come straight from the engine's own scoring, so what you see is what the engine did.
- It does **not** block all automation, and it is **not** a production console. Browser-driver profiles depend on a local browser; the T4 tier (anti-detect toolchains with real-human click-farms) is a design boundary the catalog does not resolve; and HR-11/HR-12 are heuristics that can challenge some real humans. The page labels these honestly.
- It is **separate from, and never touches, the authenticated Ledger** — it holds no admin tokens or signing keys and serves only public detection output.

## Related

- **[Self-validation: red-team your own deployment](self-validation-red-team.md)** — the terminal workflow that runs the whole catalog and reports your block-rate / false-positive rate.
- **[Hard rules and verdicts](../reference/hard-rules-verdicts.md)** — what each hard rule and verdict means.
- **[Concepts & glossary](../concepts/how-gate-sees-a-request.md)** — the L1→L7 pipeline and the risk-score → verdict model the page visualizes.
