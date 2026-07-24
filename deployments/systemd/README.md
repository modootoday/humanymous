# humanymous Gate — systemd (host binary)

Run Gate directly on a Linux host, no container. For most adopters the published
container image (see [`../compose.release.yaml`](../compose.release.yaml) or
[`../k8s/`](../k8s/)) is the easier path; this unit is for hosts where you want
the raw binary under systemd.

## 1. Build the binary

```bash
make gate                 # produces bin/gate (or: go build -o bin/gate ./cmd/gate)
sudo install -m 0755 bin/gate /usr/local/bin/gate
```

## 2. Create the service user and data dir

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin humanymous
sudo install -d -o humanymous -g humanymous -m 0750 /var/lib/humanymous
```

The unit's `ReadWritePaths=/var/lib/humanymous` is the only writable location;
the keystore and audit WAL land under it.

## 3. Write the env file

```bash
sudo install -d -m 0755 /etc/humanymous
sudo tee /etc/humanymous/gate.env >/dev/null <<'EOF'
# Seals the persistent keystore/audit identity across restarts. Keep secret.
HMN_UNSEAL=REPLACE-ME-with-a-long-random-passphrase
# Real role:token admin bearer pairs — the binary refuses the e2e-* dev tokens.
HMN_ADMIN_TOKENS=auditor:REPLACE-ME,operator:REPLACE-ME,approver:REPLACE-ME,dpo:REPLACE-ME
EOF
sudo chown root:humanymous /etc/humanymous/gate.env
sudo chmod 0640 /etc/humanymous/gate.env
```

## 4. Install and start the unit

```bash
sudo install -m 0644 humanymous-gate.service /etc/systemd/system/humanymous-gate.service
sudo systemctl daemon-reload
sudo systemctl enable --now humanymous-gate.service
systemctl status humanymous-gate.service
```

## Notes

- Edit `-upstream` in the unit to point at your origin. The shipped default is
  `http://127.0.0.1:8080`.
- The admin plane binds `127.0.0.1:8445` — reach it over SSH tunnel/mTLS only.
- The edge listens on `:8444`. To serve `:443` directly, either add ACME flags
  (`-acme-domain …`) or a TLS LB in front, and grant `CAP_NET_BIND_SERVICE`
  (commented in the unit) rather than running as root.
- `TimeoutStopSec=30` is `≥` the binary's `-shutdown-grace` (default 25s) so the
  drain completes before SIGKILL. Liveness `GET /__hmn/healthz`, readiness
  `GET /__hmn/readyz` (503 during drain).
