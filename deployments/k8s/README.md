# humanymous runtime on Kubernetes

A minimal, hardened manifest set for the Rust runtime image
(`ghcr.io/modootoday/humanymous-runtime:latest`, built from
`build/rust/runtime-candidate.Dockerfile`). It replaces the legacy Go Gate
manifests, which ran `ghcr.io/modootoday/humanymous-gate` with flags. Values
mirror [`../compose.release.yaml`](../compose.release.yaml) and
[`../systemd/`](../systemd/).

**Read the limits first.** The Rust runtime is not a drop-in replacement for
the Go Gate edge. It is a loopback-only request-integrity gateway: it listens
on loopback, reaches the origin on loopback, has no TLS, ACME, admin plane or
health endpoint, and every inventoried route requires request integrity
(`rit`) from a humanymous client SDK. The image is a candidate component, and
full product qualification is false. The two blockers in
[No Rust equivalent](#no-rust-equivalent) mean this Deployment cannot run
unattended today.

| File | What it is |
|---|---|
| `deployment.yaml` | One pod: the `runtime` container (scratch image, non-root, read-only rootfs, stdin kept open once for the configuration) and an `edge` placeholder for your TLS front proxy. |
| `service.yaml` | Public edge Service (`443 → edge:8443`). No admin Service. |
| `pvc.yaml` | `hmn-data` at `/var/lib/humanymous`: audit WAL (`audit/`) and sealed custody (`keys/`). |
| `secret.example.yaml` | STUB holding `runtime.json`, the configuration document. **Replace the placeholders before applying.** |

The Go `configmap-routes.yaml` is gone: routes are now part of the private
configuration document, which carries a secret and so cannot live in a
ConfigMap.

## Apply

```bash
# 1. Storage first. Provision sealed custody on it (see "No Rust equivalent").
kubectl apply -f pvc.yaml

# 2. The configuration Secret, ONLY after you replace the placeholders (or,
#    better, have a secrets manager materialize a Secret named
#    `humanymous-runtime-configuration`).
$EDITOR secret.example.yaml
kubectl apply -f secret.example.yaml

# 3. The workload and the Service. Replace the `edge` image first.
kubectl apply -f deployment.yaml -f service.yaml

# 4. Feed the configuration to the waiting runtime container. Repeat this
#    after every container start (restart, reschedule, rollout).
kubectl get secret humanymous-runtime-configuration -o jsonpath='{.data.runtime\.json}' \
  | base64 -d \
  | kubectl attach -i "$(kubectl get pod -l app.kubernetes.io/name=humanymous-runtime -o name)" -c runtime
kubectl logs deploy/humanymous-runtime -c runtime | head -n 1   # readiness JSON
```

## Configure the runtime

Edit `runtime.json` (the same document as
[`../humanymous-runtime.example.json`](../humanymous-runtime.example.json)):

- `listen`: `127.0.0.1:8444`. Must be loopback; the edge forwards to it.
- `origin`: `127.0.0.1:8080`. Must be a loopback socket address in this pod:
  run your app as a container in the pod, or have the edge proxy forward that
  port to your app's Service.
- `scope`: installation, tenant, origin and revision identifiers, and the exact
  `authority` (Host, port included when not default) your clients send.
- `routes`: the complete inventory of protected routes, exact method and path.
  Paths it does not list get `409 route_inventory`, so the edge must send
  health checks and static assets straight to the origin.
- `audit`: WAL and custody directories under the PVC, `node_id`, and the custody
  unseal `passphrase`. Set `allow_new` to `false` after the first successful
  start so an empty WAL fails closed instead of starting a new history.
- `owner.uid`: 65532, the pod's `runAsUser`.
- `session_capacity`, `session_ttl_ms`, `timeout_ms`, `rate_per_second`,
  `session_rate_per_second`: operational budgets (no Go equivalent).

The runtime parses the document with unknown fields refused and runs
`validate()` before it binds anything; any mistake ends the container.

## Go flag mapping

| Go `gate` flag or variable | Rust runtime configuration |
|---|---|
| `-addr :8444` | `listen` (`127.0.0.1:8444`). Loopback only; the public bind moves to your edge proxy. |
| `-admin-addr :8445` | None. No admin listener. |
| `-upstream http://origin:80` | `origin` (`127.0.0.1:8080`, a loopback socket address, no scheme or DNS name) and `scope.authority` (exact Host). |
| `-acme-domain`, `-acme-cache` | None. No ACME; certificates belong to your edge proxy. |
| `-tls-cert`, `-tls-key` | None in the binary. TLS terminates at your edge proxy. |
| `-routes /etc/hmn/routes.conf` (prefix → preset) | `routes` (exact method and path, plus `required` evidence planes). The presets `strict`, `balanced`, `monitor` and `off` have no equivalent. |
| `-keystore /data/keystore` | `audit.keys.directory` (`/var/lib/humanymous/keys`), existing sealed custody only. |
| `-audit-wal /data/audit` | `audit.directory` (`/var/lib/humanymous/audit`), with `audit.node_id`, `audit.allow_new`, `audit.trusted_head`. |
| `-shutdown-grace` (25s) | None in the standalone binary. |
| `HMN_UNSEAL` | `audit.keys.passphrase`, inside the stdin document, never in the environment. |
| `HMN_ADMIN_TOKENS` | None. No admin plane. |
| `GET /__hmn/healthz`, `/__hmn/readyz` | None. One readiness JSON line on standard output after the listener binds. |

## No Rust equivalent

These Go behaviours have no Rust counterpart today. No flag was invented for
them.

- **Configuration from a mounted file (blocker).** The binary reads its
  configuration only from standard input and refuses arguments. Kubernetes
  cannot pipe a Secret into a container's stdin, so a person (or your own
  automation) must run `kubectl attach -i` after every container start. Until
  then the container waits and serves nothing, and Kubernetes reports it
  Running. The CLI's `humanymous runtime-host --plan` reads a file, but on Linux
  its unseal secret backend is unsupported and its plan accepts only
  `humanymous-lab-` service names.
- **Custody provisioning (blocker).** The runtime opens existing sealed custody
  (`keys/keys.sealed`) and never creates it. The Go Gate created its keystore
  from `HMN_UNSEAL` on first boot. No shipped Rust command provisions custody
  yet; the only provisioning code is the Docker lab supervisor.
- **Public listener, TLS, ACME.** `listen` and `origin` must be loopback, and the
  binary serves plain HTTP/1.1 on loopback. A front proxy in the same network
  namespace is required.
- **Admin plane and admin tokens.** No listener and no RBAC tokens.
- **Health and readiness endpoints.** No HTTP probe target. The image is
  `FROM scratch`, so exec probes are impossible too.
- **Graceful drain.** No SIGTERM handler; the container stops at once. Audit
  appends are synced before a request reaches the origin, so a stop loses
  in-flight requests, not committed records.
- **Route presets and prefix matching.** Exact routes only; unlisted paths are
  refused rather than passed through.

## Admin-plane note

The Go manifests kept the admin console on a ClusterIP Service and warned that
ClusterIP is not an authorization boundary. The Rust runtime has no admin
console, so there is no admin Service. If a later runtime adds one, keep it off
every public entry point and front it with a NetworkPolicy plus mTLS or SSO.

## Scaling note

`replicas: 1` with `strategy: Recreate`. The PVC is `ReadWriteOnce` and sealed
custody takes an exclusive lock, so two pods can never share one audit
identity. To scale out, give each pod its own PVC, custody and `audit.node_id`.

## What was not verified

A Rust test (`crates/sdk/humanymous-runtime/tests/gateway/deployment_manifests.rs`)
checks that the embedded `runtime.json` matches the shared example, that it
deserializes into the runtime's `Configuration` and passes `validate()`, and
that these manifests name the Rust image and not the Go Gate. Nobody has applied
them to a cluster: `kubectl apply`, `kubectl attach -i` delivering EOF with
`stdinOnce`, the volume-root ownership checks on a real StorageClass and the
edge proxy wiring are unverified.
