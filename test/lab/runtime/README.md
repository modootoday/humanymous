# Real runtime process pilot

Run from the repository root:

```text
docker compose -f deployments/lab/compose.runtime.yaml --profile runtime build
docker compose -f deployments/lab/compose.runtime.yaml --profile runtime run --rm --no-deps runtime-pilot
```

This Rust pilot drives the actual runtime executable over private startup pipes
and real loopback TCP, then reopens its actual authenticated audit WAL. It imports
the shared runtime canonicalization and integrity library. No detector, scoring
policy or cryptographic protocol is copied into the driver. A separate private
Cargo workspace keeps Linux-only lab tooling out of published product packages.
Both lockfiles are required in the build context.

The Compose service has no network, published ports, host volumes or external
targets. It runs as UID/GID65532 with no capabilities, a read-only filesystem and
a bounded private temporary filesystem. Runtime credentials and reports are
synthetic and stay in memory. Output contains only checks, counts and limitations.
Each run creates a new temporary audit identity using public synthetic fixture
keys; these are never deployment keys.

A passing fixture proves the bounded process/session/RIT/audit/origin path. It
does not prove real-browser/human acceptance, authenticated collector, TLS/H2,
Pass recovery, assistive technology, independent witness or independent origin
bypass isolation. This process profile must not satisfy the full product e2e
gate while those requirements remain absent. Governed by
`sot:root/rust-migration` and `sot:root/runtime-application`.
