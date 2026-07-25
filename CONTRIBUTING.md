# Contributing to humanymous

Thanks for your interest in improving humanymous — a defensive, reference-build
bot-detection engine and reverse-proxy enforcement layer (Gate). This guide covers
how to build and test locally, what a pull request must pass, our commit convention,
and the one policy that surprises most first-time contributors: **detection weights and
thresholds are frozen upstream**.

By participating you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).
Security vulnerabilities must **not** be filed as normal issues or PRs — follow
[SECURITY.md](SECURITY.md) instead (private disclosure).

---

## Prerequisites

- **Go 1.25+** (the module targets `go 1.25`; CI builds on `1.25`).
- **Docker** with the Compose plugin (`docker compose`) — for the local detector-vs-bots
  lab and the Gate conformance suite.
- **Git Bash** if you are on Windows: the `Makefile` uses POSIX syntax and forward
  slashes. `make` is not required — every target maps to a plain command you can run
  directly (shown below).

---

## Build and test locally

Clone, then build and test the whole module:

```bash
git clone https://github.com/modootoday/humanymous.git
cd humanymous

# 1. Build everything (server, Gate, report, and packages)
go build ./...

# 2. Run the unit tests (all packages)
go test ./...
#   with the race detector:
go test -race ./...

# 3. Build the browser (WASM) detection engine — only compiles under GOOS=js,
#    so a plain `go build ./...` will NOT catch a break here. Always run it.
GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/

# 4. Bring up the Docker detector-vs-bots lab (engine + origin + Gate)
docker compose -f deployments/compose.yaml up -d --build core origin gate
```

Equivalent `make` targets (run under Git Bash): `make build`, `make test`, `make race`,
`make wasm`, `make up`.

### Local validation targets

Before opening a PR, run the same checks CI runs, locally:

| Target | Command | What it validates |
|---|---|---|
| `make attack` | `docker compose -f deployments/compose.yaml run --rm bots` | The automation catalog (bots) is run against the engine — all must be blocked, no false positives. |
| `make gate-e2e` | `docker compose -f deployments/compose.yaml run --rm gate-e2e` | The Gate reverse-proxy conformance suite (**34 checks**). |
| `make swarm` | `docker compose -f deployments/compose.yaml --profile swarm up --build --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c` | Multi-subnet cross-session correlation (one fingerprint over three real subnets) fires `l5.correlation.proxy_rotation`. |

The network-layer detection (TLS/HTTP-2 fingerprinting, cross-subnet correlation)
**cannot** be exercised over loopback — it only fires across a real network boundary,
which is why these run in Docker.

---

## What a pull request must pass

Every PR runs two workflows. All checks must be green before review:

**`ci.yml`**
- `go build ./...`
- `go vet ./cmd/... ./internal/...`
- `go test ./...`
- **licence-index drift check** (`scripts/check-licenses.sh`) — every module compiled into
  the shipped binaries must appear in the third-party licence index.
- **WASM build + vet** under `GOOS=js GOARCH=wasm` (`./cmd/wasm/`, `./internal/detect/`).
- **govulncheck** — reachability-aware scan for known-vulnerable dependencies.
- **git-cliff changelog-config check** — validates `cliff.toml` (release notes are
  generated from it).
- **Docker `detector-vs-bots` integration gate** — builds the images (with Trivy
  HIGH/CRITICAL image scans), runs the bots against the engine and asserts all are blocked
  with no FP, runs the Gate 34-check conformance, asserts the multi-subnet swarm fires
  `proxy_rotation`, and stands up the feature overlays.

**`codeql.yml`**
- **CodeQL** static analysis (SAST) for Go.

Please make sure `go build ./...`, `go test ./...`, and the WASM build all pass locally
before you push — they are the fastest checks and the most common cause of a red PR.

---

## Commit and PR conventions

- **Conventional Commits.** Commit subjects must follow the
  [Conventional Commits](https://www.conventionalcommits.org/) format
  (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`, …). Release notes and the
  changelog are **generated automatically** by git-cliff from these subjects, so a
  non-conforming subject line degrades the changelog. Use a scope where it helps, e.g.
  `feat(gate): …`, `fix(pass): …`.
- **Keep PRs focused.** One logical change per PR. Update any docs affected by the change
  in the same PR.
- Fill in the pull-request template checklist.

---

## Detection freeze — do not change upstream weights or thresholds

> [!IMPORTANT]
> **Detection weights, hard-rule thresholds, and verdict cut-offs are FROZEN upstream.** Do not change them in a PR — it will not be merged. Tune detection in a **fork**, or propose a **new signal** instead of re-weighting an existing one.

The detection posture (signal weights, hard-rule thresholds, and verdict cut-offs) is
**FROZEN upstream**. This is deliberate: the reference build's low-false-positive profile
and the full bots-vs-engine catalog are validated against these exact values, and changing
them silently would invalidate that evidence and the compliance documentation that cites
the design.

What this means for a contribution:

- **Do not** modify existing detection weights, thresholds, or verdict cut-offs in a PR to
  this repository. Such PRs will not be merged.
- **To tune detection for your own deployment, extend in a fork.** The weights and route
  strictness are configuration/policy seams precisely so an operator can adjust them
  without touching upstream — do that in your own build.
- **To contribute detection improvements, propose a NEW signal** rather than re-weighting
  an existing one. Open an issue describing the new signal (what it detects, its
  false-positive profile, and where it sits in the L1–L7 layering) so it can be discussed
  and validated against the catalog before any code lands. New signals are welcome;
  re-weighting the frozen ones is not.

Bug fixes, new tests, documentation, tooling, performance work, and new signals that keep
the existing catalog passing are all welcome. When in doubt, open an issue first.

---

## Reporting security issues

> [!CAUTION]
> **Do not open a public issue or PR for a security vulnerability.** Follow the private disclosure process in [SECURITY.md](SECURITY.md).
