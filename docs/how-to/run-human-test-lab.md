---
layout: doc
title: Run the human tester lab on your own PC
permalink: /how-to/run-human-test-lab.html
reviewed: 2026-11-17
---
# Run the human tester lab on your own PC

**Diátaxis quadrant:** How-to. **Audience:** the repository owner or another
authorized tester who checks the lab by hand with a real browser, keyboard and
assistive technology.

## Limits first

- This is a person's attestation, not a measured product run. A record from
  this lab never counts as an automated result, never qualifies the full
  product and never makes a release ready. An automated browser test is not a
  human baseline, so nothing here drives a browser for you.
- A lab TLS terminator in front of the Gate owns your browser's TLS handshake.
  The Gate sees plain HTTP over loopback, so this lab says nothing about raw
  TLS or HTTP/2 observation. Through the optional Cloudflare Tunnel, Cloudflare
  terminates and re-originates TLS, so those facts belong to the provider.
- The Gate runs without its independent audit witness and without the
  authority service. The witnessed topology needs the lab runner's private
  broker and is not part of this profile.
- Only your own containers are targets. Do not point the tunnel or any tool at
  third-party sites.
- **What works today:** building and starting the Gate and its origin, trusting
  the lab certificate authority, reaching the Gate over HTTPS from your browser
  and seeing it refuse what it does not know (fail-closed).
- **What waits on the Runtime Pass port:** the `/pass` page and the
  `/api/pass/*` calls. The lab already forwards those paths to the Gate, but
  the Gate has no Pass routes yet and its lab configuration has no Pass policy,
  so today every Pass step is recorded as `not_run`. Once the port lands, the
  same lab serves the Pass page with no change to the forwarding.

## Before you start

- Windows 10 22H2 or Windows 11 with Docker Desktop using the WSL 2 backend and
  its Linux engine (context `desktop-linux`). About 10 GB of free disk space
  for the first build.
- A checkout of this repository on the branch you want to test, with LF line
  endings. Note the commit: `git rev-parse HEAD`.
- Free host port 8443.
- The Rust 1.99.1 toolchain, only to turn your record into a receipt at the
  end.
- The browser and assistive technology you want to test, for example Microsoft
  Edge or Firefox with Narrator (built in) or NVDA.

Check that you are talking to the Docker Desktop Linux engine. The endpoint
must be `npipe:////./pipe/dockerDesktopLinuxEngine` and the OS type `linux`.

```powershell
docker --context desktop-linux context inspect desktop-linux --format '{{.Endpoints.docker.Host}}'
docker --context desktop-linux info --format '{{.OSType}} {{.ID}}'
```

On a Linux or macOS host, use your own engine:

```bash
docker info --format '{{.OSType}} {{.ID}}'
```

## Build and start the lab

Run these from the repository root. The first build compiles the Rust Gate and
takes a while.

```powershell
$compose = 'deployments/lab/compose.human-test.yaml'
docker --context desktop-linux compose -f $compose config --quiet
docker --context desktop-linux compose -f $compose build gate
docker --context desktop-linux compose -f $compose up -d
docker --context desktop-linux compose -f $compose ps
```

```bash
compose=deployments/lab/compose.human-test.yaml
docker compose -f "$compose" config --quiet
docker compose -f "$compose" build gate
docker compose -f "$compose" up -d
docker compose -f "$compose" ps
```

Wait until `gate` and `origin` show `healthy` and `edge` shows `running`. Only
`127.0.0.1:8443` is published; nothing listens on your network interfaces.

## Trust the lab certificate authority

The `edge` service creates a new certificate authority the first time this
lab starts. Copy out only its public root certificate, check it, then trust it
for your user account.

```powershell
docker --context desktop-linux compose -f $compose cp edge:/data/caddy/pki/authorities/local/root.crt .\humanymous-human-test-root.crt
$thumb = (Get-PfxCertificate .\humanymous-human-test-root.crt).Thumbprint
$thumb
Import-Certificate -FilePath .\humanymous-human-test-root.crt -CertStoreLocation Cert:\CurrentUser\Root
```

Windows asks for confirmation and shows a thumbprint; it must match `$thumb`.
Keep the thumbprint for clean-up. Edge and Chrome use this store. In Firefox,
open Settings, Privacy & Security, Certificates, View Certificates,
Authorities, Import, select the file and tick "Trust this CA to identify
websites".

```bash
docker compose -f "$compose" cp edge:/data/caddy/pki/authorities/local/root.crt ./humanymous-human-test-root.crt
openssl x509 -in ./humanymous-human-test-root.crt -noout -subject -fingerprint -sha256
# Chromium-family browsers on Linux (NSS user database):
certutil -d "sql:$HOME/.pki/nssdb" -A -t "C,," -n humanymous-human-test -i ./humanymous-human-test-root.crt
```

On macOS, use Keychain Access to import the file into your login keychain and
set it to "Always Trust" for SSL.

## Open the lab in your browser

1. Open `https://localhost:8443/`. The page loads with no certificate warning
   and shows a short JSON refusal such as `{"error":"route_inventory"}`. That
   is the Gate refusing a path that is not in its route inventory. Record
   `ca-trusted` and `gate-reached`.
2. Open `https://localhost:8443/pass`. Today you see the same kind of refusal;
   that is expected until the Pass port lands. After it lands, you see the
   Pass page and continue with the steps below.

Do not add exceptions for certificate warnings. A warning means the trust step
did not work; fix that instead.

## Test the Pass path by hand

Run each step on a fresh visit to `https://localhost:8443/pass`. There is no
speed requirement: take your time, pause between actions, and never treat
"too slow" as a correct result. If a step cannot be completed, record what
happened rather than retrying until it passes.

- **ordinary-pass:** complete the challenge the way you normally use a
  browser. Expected: you are allowed through.
- **pass-keyboard-only:** do not touch the mouse or touchpad. Use Tab,
  Shift+Tab, arrow keys, Enter and Space only. Check that focus is always
  visible, the order makes sense, every control is reachable and operable, and
  the result is announced on screen. Expected: you are allowed through.
- **pass-screen-reader:** start Narrator (Windows logo key+Ctrl+Enter) or NVDA
  (Ctrl+Alt+N). Listen for the page title, the instructions, each control's
  name and state and the final result. Use the screen reader's own navigation
  as well as Tab. Expected: everything needed is announced and you are allowed
  through without sight of the screen.
- **pass-zero-pointer:** complete it with no pointer movement at all, and wait
  at least 30 seconds between actions, as a switch or voice user might.
  Expected: you are allowed through; no timeout or error blames you for the
  input method or pace.

Stop the screen reader when you are done: Windows logo key+Ctrl+Enter for
Narrator, Insert+Q for NVDA.

## Record the result

Copy `test/lab/human-control-record.example.json` to a private folder outside
the repository, for example
`~/humanymous-human-test/`, and fill it in by hand. Its shape is defined in
`test/lab/human-control-record.schema.json`.

- `date` is the test day only; `source_sha` is the commit you built.
- `venue` is `local`, or `provider-fronted` for the tunnel option.
- List the operating system, browser and assistive technology with versions,
  and the input methods you used.
- Give every step an outcome: `passed`, `failed`, `blocked` or `not_run`.
  Today the four Pass steps are `not_run`.
- Notes are optional and short. Never write names, email addresses, account
  identifiers, links or anything you saw in another window.

Then turn the record into a receipt in an interactive terminal. The command
refuses to run in CI or without a terminal, asks you to confirm in person and
never overwrites an existing receipt.

```powershell
cargo run --locked -p humanymous-lab-runner -- human-control-receipt --record $HOME\humanymous-human-test\record.json --emit $HOME\humanymous-human-test\receipt.json
```

```bash
cargo run --locked -p humanymous-lab-runner -- human-control-receipt --record "$HOME/humanymous-human-test/record.json" --emit "$HOME/humanymous-human-test/receipt.json"
```

The receipt is labelled `human-attested/local` or
`human-attested/provider-fronted` and reports `attested_complete` only when
every step passed. It never reports `passed`, never sets
`full_product_qualified` and does not clear the lab's list of unavailable
human and assistive controls; accepting it into a product decision is the
owner's call. Keep the record and receipt private unless they are sanitized.

## Optional: reach the lab through Cloudflare Tunnel and Access

Use this only to test from another device you own. You need a Cloudflare
account with a zone you control and Cloudflare Zero Trust enabled. Dashboard
menu names change over time; the steps describe what to set, not exact labels.

1. **Create the tunnel.** In Zero Trust, under Networks, create a
   `cloudflared` tunnel named, for example, `humanymous-human-test`. Copy its
   token from the install command it shows. Do not run that install command.
2. **Create the Access application first.** Under Access, add a self-hosted
   application for one exact hostname, for example
   `hm-human-test.<your zone>`. Add one Allow policy that includes only your
   own email address (or a specific Access group). Use a short session
   duration such as one hour. Do not add Bypass or Everyone rules.
3. **Add the public hostname to the tunnel.** Route that exact hostname to
   service type HTTP, URL `gate:8080`. Keep the catch-all rule returning 404.
   In the hostname's Access settings, turn on Access protection and select the
   application from step 2, so the connector also checks the Access token.
   Never route any other service or path.
4. **Start the tunnel with the token in your session only.** The token is read
   from the `HM_TUNNEL_TOKEN` variable; never put it in a file in the
   repository. `docker compose config` prints it while it is set, so do not
   share that output.

```powershell
$env:HM_TUNNEL_TOKEN = Read-Host -MaskInput 'Tunnel token'
docker --context desktop-linux compose -f $compose --profile tunnel up -d
docker --context desktop-linux compose -f $compose --profile tunnel ps
```

```bash
read -rs -p 'Tunnel token: ' HM_TUNNEL_TOKEN && export HM_TUNNEL_TOKEN && echo
docker compose -f "$compose" --profile tunnel up -d
docker compose -f "$compose" --profile tunnel ps
```

5. Open `https://hm-human-test.<your zone>/` from your other device. You must
   see the Cloudflare Access sign-in first; if you reach the Gate without it,
   stop the tunnel and fix the Access application. The browser trusts
   Cloudflare's certificate there, not the lab CA.
6. Repeat the steps above and record `venue` as `provider-fronted`.

The tunnel container joins only the Gate-facing network and its own egress
network. It has no access to the origin network, the certificate authority
volume, the host network or Docker itself. The hop from the tunnel container
to the Gate is plain HTTP on a private Docker network that is not published.

## Stop and clean up

`down --volumes` deletes the lab certificate authority too, so the next start
creates a new one that you trust again.

```powershell
Remove-Item Env:\HM_TUNNEL_TOKEN -ErrorAction SilentlyContinue
docker --context desktop-linux compose -f $compose --profile tunnel down --volumes --remove-orphans
Remove-Item "Cert:\CurrentUser\Root\$thumb"
Remove-Item .\humanymous-human-test-root.crt
docker --context desktop-linux image rm humanymous/product-e2e:local
```

```bash
unset HM_TUNNEL_TOKEN
docker compose -f "$compose" --profile tunnel down --volumes --remove-orphans
certutil -d "sql:$HOME/.pki/nssdb" -D -n humanymous-human-test
rm ./humanymous-human-test-root.crt
docker image rm humanymous/product-e2e:local
```

In Firefox, remove the imported authority from the Authorities list as well.
If you used the tunnel, delete its public hostname, the tunnel itself (this
revokes the token) and the Access application in the Cloudflare dashboard.
You can also remove the pulled `caddy` and `cloudflare/cloudflared` images.

## Related

- [Accessible recovery](../help/challenge-accessibility.md)
- [Supported topologies](../reference/supported-topologies.md)
- [Plan TLS termination](https-tls-certificates.md)
- [Owned lab evaluation](self-validation-red-team.md)
