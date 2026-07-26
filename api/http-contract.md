# HTTP API contract — standalone detection engine

This is the wire contract between the browser client and the standalone Core
detection engine in `cmd/server`. All responses are JSON unless noted. Core
terminates TLS itself so it can observe the connection fingerprint and HTTP/2
behavior.

Core and the Gate reverse proxy are different runnable surfaces. Core exposes
the browser-report API below. Gate uses the same scoring package but has a
smaller request-time evidence set and exposes its operator API under
`/__hmn/admin/`. Do not assume a Core result and a Gate result contain identical
evidence.

## Pages

| Method | Path | Purpose |
|---|---|---|
| GET | `/` | Load the detector demo shell and set the `hsid` session cookie. |
| GET | `/demo` | Open the public, read-only “score this browser” page. |
| GET | `/static/*` | Serve the WebAssembly detector, JavaScript modules, and supporting assets. |

## API

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/session` | Issue or confirm a session and return browser-hint headers plus request-integrity state. |
| POST | `/api/collect` | Submit a browser report, combine it with the connection observation, score it, and return the verdict. |
| GET | `/api/report/{id}` | Return the full report for one session. |
| GET | `/api/report` | Return recent session summaries. |
| POST | `/api/pow` | Submit proof of completed computational work for an eligible challenge. |
| POST | `/api/csp-report` | Record a Content Security Policy violation report as an integrity signal. |
| GET | `/api/trace` | Return the scoring trace used by the local Detection Observatory. |

### `GET /api/session` → 200

```json
{ "sessionId": "…", "ritOn": true, "ritSeed": "base64url", "ritN": 0, "ritW": 10 }
```

The `rit*` properties are retained wire names for the request-integrity token:
they tell the browser whether protection is active, provide the current seed,
and identify the accepted counter window. The response also advertises the
browser client hints the detector consumes:

```http
Accept-CH: Sec-CH-UA-Full-Version-List, Sec-CH-UA-Platform-Version, Sec-CH-UA-Arch
```

### `POST /api/collect`

The request body is the `ClientReport` defined in `internal/signals`. It includes
the user agent, collected browser signals, environment and advanced inspection
results, integrity observations, interaction summaries, and a browser
fingerprint identifier. When request-integrity protection is active, the client
signs the submission with the token headers returned for the session.

Response → 200:

```json
{
  "sessionId": "…",
  "riskScore": 0,
  "verdict": "ALLOW",
  "hardRuleFired": null,
  "topContributors": [
    { "id": "l1.example.signal", "score": 0 }
  ],
  "policyVersion": "1.0.0"
}
```

`verdict` is one of `ALLOW`, `CHALLENGE`, or `DENY`. `hardRuleFired` and the
contributor `id` are legacy machine fields. Reader-facing tools must resolve
them to the descriptive rule and signal names from the
[glossary and rule reference](../docs/reference/glossary.md), rather than show
the identifiers alone.

The response may rotate the request-integrity seed through `X-HM-Seed`,
`X-HM-N`, and `X-HM-W`. An eligible score-based challenge may also include an
`X-HM-PoW` header describing the computational-work request. Completing that
work does not prove a person is present and cannot override a mandatory-rule
denial.

### `GET /api/report/{id}` → 200

The full `SessionReport` contains:

- `client.signals[]`: evidence collected in the browser;
- `network`: connection fingerprint, header order, and network observations;
- `crosschecks[]`: consistency checks across the browser claim, browser
  environment, and connection;
- `scoring`: the score and verdict described above; and
- `label`: the optional evaluation label.

Exact property names remain stable for API consumers. Interfaces should present
their descriptive glossary names.

## Verdict bands

With the built-in policy, scores from 0 through 29 allow the request, 30 through
69 challenge it, and 70 through 100 deny it. A mandatory rule can raise the
result when a high-confidence condition is met. The current descriptive catalog
and compatibility identifiers are documented in
[Hard rules and verdicts](../docs/reference/hard-rules-verdicts.md).

The challenge-recovery path is a reference implementation with documented
accessibility and integration gaps. Treat it as an example to evaluate, not as
proof of humanity or a production-ready accessibility claim.
