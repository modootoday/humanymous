# HTTP API contract — detection engine (cmd/server)

The wire contract between the browser client and the Blue engine. Derived from
`cmd/server/handlers.go`; all responses are JSON unless noted. TLS is terminated
by the engine itself (to read JA3/JA4 + the HTTP/2 fingerprint).

## Pages
| Method | Path | Purpose |
|---|---|---|
| GET | `/` | detector demo shell (`index.html`); sets the `hsid` session cookie |
| GET | `/demo` | public read-only "score your browser" page |
| GET | `/static/*` | web assets (`detector.wasm`, JS modules, `wasm_exec.js`) |

## API
| Method | Path | Purpose |
|---|---|---|
| GET | `/api/session` | issue/confirm the session; returns Accept-CH hints + RIT seed |
| POST | `/api/collect` | submit the `ClientReport`; merge with the network observation, score, return the verdict |
| GET | `/api/report/{id}` | full `SessionReport` for a session id |
| GET | `/api/report` | recent session summaries |
| POST | `/api/pow` | submit a proof-of-work solution to upgrade a CHALLENGE |
| POST | `/api/csp-report` | CSP violation sink (records a guard signal) |
| GET | `/api/trace` | scoring trace for a session (Observatory) |

### `GET /api/session` → 200
```json
{ "sessionId": "…", "ritOn": true, "ritSeed": "base64url", "ritN": 0, "ritW": 10 }
```
Also sets `Accept-CH: Sec-CH-UA-Full-Version-List, Sec-CH-UA-Platform-Version, Sec-CH-UA-Arch`.

### `POST /api/collect`
Request body — the `ClientReport` (`internal/signals`): `userAgent`, `signals[]`
(L1–L4 client signals), `environment`, `advanced`, `guard`, `behavior`,
`fingerprintId`. Signed with the RIT token headers when RIT is on. Response → 200:
```json
{
  "sessionId": "…",
  "riskScore": 0,
  "verdict": "ALLOW | CHALLENGE | DENY",
  "hardRuleFired": "HR-7 | null",
  "topContributors": [ { "id": "l1.…", "score": 0 } ],
  "policyVersion": "1.0.0"
}
```
Response headers may carry `X-HM-Seed`/`X-HM-N`/`X-HM-W` (rotated RIT seed) and
`X-HM-PoW` (proof-of-work challenge on a score-based CHALLENGE).

### `GET /api/report/{id}` → 200
The full `SessionReport`: `client{signals[]}`, `network{ja3,ja4,ja4Engine,headerOrder,signals[]}`,
`crosschecks[]` (L6), `scoring` (the `ScoreResult` above), `label`.

## Verdict bands
`ALLOW` 0–29 · `CHALLENGE` 30–69 · `DENY` 70–100. A hard rule (HR-*) can override
the band. See `docs/reference/` for the signal and hard-rule catalogs.

> The Gate proxy (`cmd/gate`) speaks the same scoring model at the edge
> under its `/__hmn/` control path; its admin API is documented in the console.
