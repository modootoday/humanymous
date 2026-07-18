# configs

Environment and policy configuration for running the stack.

- **`dev.env`** — dev-only settings loaded by the local Docker stack
  (`deployments/compose/*.yaml` reference it via `env_file`). Holds the
  deterministic Gate admin tokens, the Red catalog target, and the shared
  fingerprint used by the multi-subnet swarm. **Not for production** — production
  issues admin credentials via mTLS/SSO and mints secrets out-of-band.

Runtime policy that is currently compiled in (the engine's scoring bands and the
Gate route sensitivity map) is documented under `docs/reference/` and, for
the Gate routes, set on the `gate` command in the compose fragment.
