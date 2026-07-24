---
description: "humanymous Gate install requirements: Go 1.25.3, one direct dependency (uTLS), pure Go with no CGO, cross-compiles to Linux, macOS, Windows. Reference build needs no database or Redis."
keywords: ["humanymous Gate install requirements","Go 1.25.3 toolchain","refraction-networking/utls uTLS dependency","pure Go no CGO build","GOOS GOARCH cross-compile","bot detection self-hosted install","reference implementation not production-hardened","edge and admin listener ports 8444 8445","no database Redis or KMS","distroless container build"]
---

# Install, system requirements & supported platforms

> **Diátaxis quadrant:** Reference. **Audience:** integrators standing up humanymous Gate for the first time (step zero).

This page lists exactly what you need to build and run the reference implementation of humanymous Gate: the toolchain, the one dependency, the build command, the ports it listens on, and the external services it does *not* need. It is a lookup, not a walkthrough — when you are ready to run, follow the [monitor-mode quickstart](../tutorials/quickstart-monitor-mode.md).

> **Note:** This repository is a **reference implementation**, not a production-hardened build. Several things a production deployment would need (shared fleet state, a real key-management service, CA-issued certificates, and more) are **prod-delta** and are not present here. See [production vs reference](production-vs-reference.md) for the full list.

## Toolchain

| Requirement | Value |
|---|---|
| Go toolchain | go 1.25.3 (declared in `go.mod`) |
| CGO | Not used — pure Go |
| Direct dependencies | One: `github.com/refraction-networking/utls` v1.8.2 (uTLS) |

Gate is written in pure Go with no CGO. That keeps the build simple and lets you cross-compile for any target with `GOOS`/`GOARCH` — you do not need a C compiler or platform-specific build tooling.

The Go module is `github.com/modootoday/humanymous`. Its only direct dependency is uTLS, which Gate uses to read TLS ClientHello fingerprints (part of the L5 network layer — see [how Gate sees a request](../concepts/how-gate-sees-a-request.md)).

## Supported platforms

Because the build is pure Go, Gate builds and runs on Linux, macOS, and Windows via the standard Go cross-compilation flags (`GOOS`/`GOARCH`).

The build command below writes `bin/gate.exe`. The `.exe` name reflects a Windows development host — it is just an output filename, not a platform constraint. On Linux or macOS, name the output whatever suits your OS (for example, `bin/gate`).

> **TODO(verify):** Minimum and recommended hardware sizing (RAM, CPU cores) for the edge and admin listeners is not specified in the verified facts.

> **TODO(verify):** A supported-OS/architecture matrix with specific version floors (for example, minimum Linux kernel, macOS, or Windows versions) is not specified in the verified facts.

> **Note:** Container packaging is defined in the repository under `build/` — `build/engine.Dockerfile` (the detection engine) and `build/gate.Dockerfile` (Gate), both on a `gcr.io/distroless/static-debian12:nonroot` base — and the local stack is wired in `deployments/compose.yaml`. No image is published to a public registry; build locally with `make docker` (engine) or `docker compose -f deployments/compose.yaml build`.

## Build

Build the edge binary from the repository root:

```
go build -o bin/gate.exe ./cmd/gate
```

On Linux or macOS, substitute an OS-appropriate output name, for example:

```
go build -o bin/gate ./cmd/gate
```

To cross-compile from one host for another target, set `GOOS` and `GOARCH` before the build (for example, `GOOS=linux GOARCH=amd64`). No CGO toolchain is required.

## Ports

The reference runs three localhost listeners. Two are Gate's own; the third is your origin application, which Gate fronts.

| Port | Role | Flag / source |
|---|---|---|
| `:8444` | Public edge (HTTPS) — terminates TLS, injects the detection bundle, scores L1–L7, and enforces the verdict | `-addr ":8444"` |
| `:8445` | Separate authenticated admin listener (Ledger + admin API), cross-origin to the edge | `-admin-addr ":8445"` |
| `:9000` | Your origin app that Gate proxies to (the `-upstream` target) | `-upstream "http://127.0.0.1:9000"` |

The admin listener is deliberately separate from the public edge: `/__hmn/admin/*` is not reachable on the edge and returns `404` there. The Ledger is served at `https://localhost:8445/__hmn/admin/console`. For the full flag list and defaults, see [CLI, config & policy](cli-config-policy.md).

> **Note:** These are the reference defaults. Each is configurable through the flag shown; the origin base URL in particular will point at wherever your application actually listens.

## External service dependencies

In the reference, there are **none**. Gate holds its state in memory in a single process:

- The verdict store, rate-limit counters, and bans are in-process (single node).
- The audit log, keys, and pseudonym vault live in-process (optionally sealed to a local keystore file with `-keystore` plus the `HMN_UNSEAL` passphrase — see [key management](../how-to/key-management.md)).
- The development TLS certificate is self-signed in memory.

You do not need a database, Redis, or a key-management service to run the reference.

> **Important:** Shared fleet state (for example, Redis-backed verdict store and bans), a real KMS/HSM, and CA-issued certificates are **prod-delta** — they are what a production deployment would add, and they are not part of this reference. Do not plan a multi-node deployment on the assumption that shared state exists here; it does not. See [production vs reference](production-vs-reference.md).

## First run

Once the binary builds, start with monitor mode so Gate scores and logs traffic without enforcing any verdict. Follow the step-by-step [monitor-mode quickstart](../tutorials/quickstart-monitor-mode.md), then read the [integrator start-here guide](../start-here/integrator.md) for how to place Gate in front of your origin.

## Related pages

- [Monitor-mode quickstart](../tutorials/quickstart-monitor-mode.md) — your first run, end to end.
- [CLI, config & policy](cli-config-policy.md) — every flag, preset, and route default.
- [Production vs reference](production-vs-reference.md) — the complete prod-delta list.
- [Integrator start-here](../start-here/integrator.md) — putting Gate in front of your app.
- [How Gate sees a request](../concepts/how-gate-sees-a-request.md) — the L1–L7 detection model.
