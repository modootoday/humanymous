# Write or extend a Red-team profile

**Quadrant:** How-to (task-oriented, guarded).
**Audience:** developers adding a new automation-tell profile to their own local catalog.

> **Rules of engagement.** This profile fires at your own detection engine on `127.0.0.1` only. You are reproducing the *signature* of an automation stack to test your own detector — never building evasion against a system you do not operate. Findings about coverage go to the maintainers, never optimized into a bypass. See [Red-team rules of engagement](../reference/red-team-rules-of-engagement.md).

A profile is a small, self-checking test that reproduces one automation stack's tell, fires it at your own detection engine, and asserts the verdict you expected. This page walks through writing one, classifying it so the harness scores it correctly, registering it in the three places a new file must appear, and verifying it end to end.

## Before you start

You need the standalone detection engine running on loopback. It is the same L1–L7 engine your profile will exercise. Build the detector module and the server:

```
GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/
```

```
go build -o bin/server.exe ./cmd/server/
```

Start it on `127.0.0.1:8443` and leave it running in its own shell:

```
bin/server.exe -addr 127.0.0.1:8443 -web web
```

The other half of the picture is the end-to-end runner, `test/e2e/runner.mjs`. It loads each registered profile, calls its `run(baseURL)`, records the verdict, and does the true-positive / false-negative bookkeeping. Your profile is only "done" when the runner scores it the way you intended.

> **Note:** The engine uses a self-signed dev certificate. The harness accepts it precisely because the target is local — one more reason a profile must stay pointed at `127.0.0.1`.

## Choose a class

A profile belongs to one of two classes. Pick the one that matches the tell you are reproducing.

- **Browser-driven** — the tell lives in the client runtime (a leaked automation property, a driver flag, a fingerprint quirk). Use the shared helper `test/redteam/_driver.mjs`, set `needsBrowser = true`, and reproduce the tell rather than driving a real automation tool.
- **Non-browser / network** — the tell lives in the request itself (a spoofed client report, a raw-protocol behavior). Do a `fetch` POST to `/api/collect`, or wrap a raw attack, and set `needsBrowser = false`.

Every profile, in either class, satisfies the same contract. It is a file `test/redteam/<name>.mjs` that exports:

- `async function run(baseURL)` returning `{verdict, riskScore, hardRuleFired, topContributors}` — exactly the JSON that `POST /api/collect` returns and that `test/e2e/runner.mjs` reads.
- `export const label` — a short name for the sample (see [Pick a label that classifies correctly](#pick-a-label-that-classifies-correctly)).
- `export const needsBrowser` — `true` for browser-driven profiles, `false` for network ones.

## A minimal browser profile

A browser profile reproduces the *signature* of an automation stack — it does not drive a real driver. The shared helper `drive(baseURL, {headless, initScripts, ...})` launches installed Edge (`chromium.launch` channel `'msedge'`) or Playwright Firefox, runs your init scripts before page load, and returns the engine's verdict. Modeled on `selenium.mjs`, which plants the `cdc_` property and the webdriver flags via `addInitScript`:

```
import { drive } from './_driver.mjs';

export const label = 'bot:selenium-like';
export const needsBrowser = true;

export async function run(baseURL) {
  return drive(baseURL, {
    headless: true,
    initScripts: [
      // Plant the tell the way the real stack leaks it — do not drive a real driver.
      () => {
        Object.defineProperty(navigator, 'webdriver', { get: () => true });
        window.document.$cdc_asdjflasutopfhvcZLmcfl_ = {};
      },
    ],
  });
}
```

The point is fidelity to the *tell*, not to the tool: you are giving your own detector something that looks like the signature (here, an automation property such as `l1.navigator.webdriver` and the injected `cdc_` marker) so you can confirm the detector reacts.

## A minimal non-browser profile

A network profile skips the browser entirely and posts a spoofed client report straight to the collector, then returns the parsed JSON. Modeled on `http_client.mjs`:

```
export const label = 'bot:http-client-like';
export const needsBrowser = false;

export async function run(baseURL) {
  const res = await fetch(`${baseURL}/api/collect`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      // A minimal, obviously-non-browser client report.
      userAgent: 'python-requests/2.x',
      signals: {},
    }),
  });
  return res.json();
}
```

For a raw-protocol tell you can instead wrap an attack case through `test/redteam/_bin.mjs` — call `attack(name, hostOf(baseURL))` and return its verdict, modeled on `flood.mjs`. This lets a network profile reuse a `cmd/redteam` attack case (see [Raw-protocol tells go in cmd/redteam](#raw-protocol-tells-go-in-cmdredteam)) without leaving the profile catalog.

## Pick a label that classifies correctly

The label is not cosmetic — the runner's `classify()` reads it to decide how a verdict is scored.

- A label that **starts with `bot:`** marks the sample as automation. The only non-bot label is `human.mjs`, the honest-human baseline; any label that does *not* start with `bot:` is treated as a human-baseline sample, so its detections flip into false-positive / true-negative accounting.
- For a `bot:` label, `classify()` counts **CHALLENGE or DENY as a true positive** (caught) and **ALLOW as a false negative** (missed).
- For the human baseline, **DENY is a false positive**; everything else, including CHALLENGE, is a true negative.

So align three things — the label, the expected verdict, and the expected hard rule — so the profile is self-checking. A profile that reproduces an automation tell should carry a `bot:` label and expect CHALLENGE or DENY; if it returns ALLOW, that is a real outcome the runner will record as a miss, not something to paper over.

> **Important:** A label that does not start with `bot:` silently becomes a human-baseline sample. If your automation profile is accidentally mislabeled, every detection it triggers is counted as a false positive against your own numbers.

## Register in three places

This is the step people miss. A new profile file is **silently ignored** until it is registered in all three of these places — miss any one and the file simply does nothing, with no error to tell you why.

1. **The runner's profile list** — add the filename to the `PROFILES` array in `test/e2e/runner.mjs`, or the e2e suite never loads it.
2. **The launcher allowlist** — add the filename to the `launchProfiles` allowlist in `cmd/server/launch.go`, or the Detection Observatory rejects the `profileId` as unknown.
3. **The Observatory catalog** — add the filename to the catalog grouping in `web/playground.html`, or it never renders as a launcher card.

> **Note:** The `launchProfiles` allowlist is also a guard, not just a registry — the Observatory launcher will only start a profile that appears on it, which is part of what keeps the launcher structurally loopback-locked. Adding your profile there is how you opt it in; it is not a check to route around.

## Verify end to end

Verify in three passes, from smallest to fullest signal.

Run the profile on its own against your local engine:

```
node test/redteam/run-one.mjs <name>.mjs https://127.0.0.1:8443
```

Its stdout is deliberately minimal — `{label, verdict, riskScore, hardRuleFired}`. The `topContributors` and the full per-layer trace do not print here; they come from the Observatory's `/playground/explain/{id}` or from the e2e `results.json`.

Next, launch the profile in the Detection Observatory and watch it run. The L1→L7 waterfall and the "Why this verdict" hard-rule ladder should show what you expected — the tell you planted lighting up the layer you aimed at, and the verdict landing where your label assumes it will.

Finally, run the full suite and confirm the bookkeeping matches your intent:

```
node test/e2e/runner.mjs
```

A `bot:` profile you expected to be caught should read as a true positive (CHALLENGE or DENY); if it reads as a false negative (ALLOW), the runner is telling you the truth about a coverage gap — see [Interpret and act](#interpret-and-act).

## Raw-protocol tells go in cmd/redteam

Some tells cannot be expressed as a browser profile at all — mid-session TLS rotation, request-integrity-token replay or tamper, floods, distributed patterns. For those, reach for `cmd/redteam` (Go, uTLS) and add a new `-attack` case there instead of a browser profile. A network profile can then surface it in the catalog by wrapping it through `_bin.mjs` as shown above.

> **Warning:** `cmd/redteam`'s `-host` default (`127.0.0.1:8443`) is **advisory, not enforced** — the CLI connects to whatever host you pass, so keeping it on your own box is your responsibility. This is unlike the Observatory launcher, which is structurally loopback-locked. Never pass a host you do not operate.

## Interpret and act

Read an unexpected ALLOW as data about your own coverage, not as a target.

- An ALLOW where the profile should have been caught is a **coverage gap**. Raise it with the maintainers, and fix it on the Blue side via [Extend detection](extend-detection.md) — add or strengthen the signal so the engine reacts to the tell.
- Never tune the profile to defeat your own detector. The profile exists to *reveal* gaps; a profile edited until it slips through tells you nothing.
- Never weaken the self-target-only guards: `run-one.mjs`'s loopback check, the server-fixed launch base, and the `launchProfiles` allowlist. Those keep the whole exercise defensive-only.

> **Note:** A change that makes a profile pass is only progress if it is a change to the *detector* that closes the gap — not a change to the profile that hides it.

## Related

- [Red-team catalog](../reference/red-team-catalog.md) — the existing profiles and what each one reproduces.
- [Self-validation: red-team your own deployment](self-validation-red-team.md) — running the full catalog to measure your own detection and false-positive rates.
- [Red-team rules of engagement](../reference/red-team-rules-of-engagement.md) — the defensive-only boundary every profile lives inside.
