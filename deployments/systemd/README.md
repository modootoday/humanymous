# humanymous runtime: systemd (host binary)

Run the Rust `humanymous-runtime` binary directly on a Linux host, no container.
This unit replaces the legacy Go `humanymous-gate.service`, which ran `gate`
with flags. The Rust binary takes **no arguments**: it reads one private JSON
configuration document on standard input, so the unit feeds it with
`StandardInput=file:/etc/humanymous/runtime.json`.

Read the limits first. The Rust runtime is a **loopback-only** gateway: it
listens on loopback, reaches the origin on loopback, has no TLS, ACME, admin
plane or health endpoint, and every inventoried route requires request
integrity (`rit`) from a humanymous client SDK. The full Go flag mapping and the
list of Go behaviour with no Rust equivalent is in
[`../k8s/README.md`](../k8s/README.md#go-flag-mapping). Full product
qualification is false.

## 1. Build the binary

```bash
cargo build --release --locked -p humanymous-runtime --no-default-features --features gateway --bin humanymous-runtime
sudo install -m 0755 target/release/humanymous-runtime /usr/local/bin/humanymous-runtime
```

These are the same features `build/rust/runtime-candidate.Dockerfile` builds.

## 2. Create the service user and private directories

```bash
sudo useradd --system --uid 65532 --no-create-home --shell /usr/sbin/nologin humanymous
sudo install -d -o humanymous -g humanymous -m 0700 /var/lib/humanymous
sudo install -d -o humanymous -g humanymous -m 0700 /var/lib/humanymous/audit /var/lib/humanymous/keys
```

The example configuration names `owner.uid` 65532, the same UID as the
container image. If you choose another UID, change `owner.uid` to match
`id -u humanymous`. The runtime refuses audit and custody directories that are
not owned by that UID with mode 0700, and refuses any ancestor that another
unprivileged user owns or can write. The unit's `ReadWritePaths=/var/lib/humanymous`
is the only writable location.

## 3. Provision sealed audit custody

The runtime opens existing custody (`/var/lib/humanymous/keys/keys.sealed`)
and never creates it. The Go `gate` created its keystore from `HMN_UNSEAL` on
first boot; **no shipped Rust command provisions custody yet**. Until one ships,
this unit cannot start on a fresh host. The only provisioning code today is the
Docker lab supervisor (`test/lab/runtime/src/product_e2e/gate.rs`), which is
not an operator tool.

## 4. Write the configuration document

```bash
sudo install -d -m 0755 /etc/humanymous
sudo install -o root -g root -m 0600 ../humanymous-runtime.example.json /etc/humanymous/runtime.json
sudoedit /etc/humanymous/runtime.json
```

Edit at least: `scope` (installation, tenant, origin, revision, and the exact
`authority` your clients send as Host), `routes`, `audit.node_id`, and
`audit.keys.passphrase` (the custody unseal passphrase; it replaces Go's
`HMN_UNSEAL`). The document must stay valid JSON with no unknown fields; the
runtime refuses anything else before it binds a listener. Set
`audit.allow_new` to `false` after the first successful start so an empty or
missing WAL fails closed instead of starting a new history.

## 5. Install and start the unit

```bash
sudo install -m 0644 humanymous-runtime.service /etc/systemd/system/humanymous-runtime.service
sudo systemctl daemon-reload
sudo systemctl enable --now humanymous-runtime.service
journalctl -u humanymous-runtime.service -o cat | head -n 1   # readiness JSON
```

If you ran the Go unit, stop and disable `humanymous-gate.service` first. Do
not point the Rust runtime at the Go keystore or WAL: no migration between them
is documented or tested.

## Notes

- **Edge.** The runtime listens on `127.0.0.1:8444` (`listen`). Run your own
  TLS front proxy on the host for `:443` and forward the inventoried routes to
  it. Paths the inventory does not list get `409 route_inventory`; send health
  checks and static assets from the front proxy straight to the origin.
- **Origin.** `origin` must be a loopback socket address such as
  `127.0.0.1:8080`, not a URL or a DNS name.
- **Admin plane.** Go's `-admin-addr 127.0.0.1:8445` has no equivalent: the
  standalone binary exposes no administration listener.
- **Shutdown.** The binary has no SIGTERM handler, so `systemctl stop` ends it
  at once without a drain. `TimeoutStopSec=30` is kept from the Go unit.
- **Health.** There is no `/__hmn/healthz` or `/__hmn/readyz`. The only
  readiness signal is the JSON line on standard output after the listener binds.

## What was not verified

This unit and README were checked by a Rust test
(`crates/sdk/humanymous-runtime/tests/gateway/deployment_manifests.rs`) that
parses the example configuration with the runtime's own `Configuration` type and
runs `validate()`. Nobody has started this unit under a real systemd with real
custody; `StandardInput=file:` ordering and the sandbox with this binary are
unverified.
