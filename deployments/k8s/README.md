# humanymous Gate on Kubernetes

A minimal, hardened manifest set for running the published Gate image
(`ghcr.io/modootoday/humanymous-gate:latest`) as a reverse-proxy edge in front
of your own origin. Values mirror [`../compose.release.yaml`](../compose.release.yaml).

| File | What it is |
|---|---|
| `deployment.yaml` | The Gate pod: distroless/non-root/read-only-rootfs, httpGet liveness+readiness probes, graceful drain, PVC + ConfigMap + Secret wiring. |
| `service.yaml` | Public edge Service (`443 → 8444`) + an internal-only admin Service (`8445`). |
| `pvc.yaml` | `hmn-data` (keystore + audit WAL) and `hmn-acme` (issued certs). |
| `secret.example.yaml` | STUB for `HMN_UNSEAL` + `HMN_ADMIN_TOKENS`. **Replace before applying.** |
| `configmap-routes.yaml` | `routes.conf` route policy. |

## Apply

```bash
# 1. Storage + route policy first
kubectl apply -f pvc.yaml -f configmap-routes.yaml

# 2. The Secret — ONLY after you replace the placeholders (or, better, have a
#    secrets manager materialize a Secret named `humanymous-gate-secrets`).
#    The binary refuses to boot on the e2e-* dev tokens, so real values are
#    mandatory or the pod crash-loops.
$EDITOR secret.example.yaml
kubectl apply -f secret.example.yaml

# 3. The workload + Services
kubectl apply -f deployment.yaml -f service.yaml
```

## Point the edge at your origin

Edit these `args` in `deployment.yaml`:

- `-upstream` → your origin Service URL, e.g. `http://app.default.svc:8080`.
- `-acme-domain` → the public domain Gate serves. Let's Encrypt (TLS-ALPN-01)
  needs port **443** reachable at that domain — hence the `LoadBalancer` edge
  Service. For **bring-your-own TLS** instead, drop `-acme-domain`, mount your
  cert, add `-tls-cert`/`-tls-key`, and you can make the edge Service `ClusterIP`
  behind an Ingress.

Edit `routes.conf` in `configmap-routes.yaml` to match your app's paths.

## Health, drain, and rollout

- Liveness: `GET /__hmn/healthz` (HTTPS, port 8444). Readiness: `GET /__hmn/readyz`,
  which returns **503 during drain** so the endpoint is pulled first.
- The image is **distroless — no shell**, so probes are `httpGet`, never `exec`.
- `terminationGracePeriodSeconds: 30` is `≥` the binary's `-shutdown-grace`
  (default 25s) so the in-process SIGTERM drain finishes before SIGKILL.
- A `preStop` sleep de-races endpoint removal. If your image variant lacks a
  `sleep` binary, delete that hook — the readiness 503 plus the SIGTERM drain
  already cover graceful shutdown.

## Admin-plane reachability caveat

The admin console binds `-admin-addr :8445` on **every pod interface**, and
`service.yaml` exposes it only as a **ClusterIP** (`humanymous-gate-admin`) — no
LoadBalancer, NodePort, or Ingress. ClusterIP is *not* an authorization boundary:
any pod that can route to it reaches the console. **Restrict it with a
NetworkPolicy** (allow only your admin namespace/pods) **and front it with
mTLS/SSO**. Never attach a public entry point to the admin Service.

## Scaling note

`replicas: 1`. The PVCs are `ReadWriteOnce` and ACME issues one cert per domain
onto the shared `hmn-acme` volume, so multiple replicas against one RWO PVC will
not work. To scale out, use an RWX StorageClass (or terminate TLS upstream and
drop `-acme-domain`) and give each pod its own keystore identity.
