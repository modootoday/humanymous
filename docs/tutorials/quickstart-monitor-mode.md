# Quickstart: Put Gate in front of an app in monitor mode (first 30 minutes)

**Tutorial** (learning-oriented) · **Audience:** integrator or evaluator, first run.

Welcome. In the next 30 minutes you will stand up **humanymous Gate** — the reverse-proxy enforcement layer — in front of a throwaway app on your own machine, running in **global monitor mode**. Monitor mode is where every deployment should start: Gate scores and logs every request across layers L1–L7, but it enforces *nothing*. No visitor is challenged, no visitor is blocked, and your origin app behaves exactly as it did before. You get to watch the detection work with zero risk to real traffic.

That monitor-first shape is deliberate. It is how the product is meant to be adopted: observe first, build confidence from the audit trail, and only then decide where to turn enforcement on. This guide follows a single path that is designed to succeed on the first try.

> **Note:** This repository is a reference implementation, not a production-hardened build. The dev TLS certificate is self-signed and generated in memory, and the admin tokens shown here are development tokens. Everything below is for a local evaluation on `127.0.0.1`.

## Before you start

You will need:

- **Go** installed (to build the binary from `./cmd/gate`).
- **Python 3** (used only for the one-line throwaway origin; any tiny static HTTP server works).
- A **web browser**.
- A shell open in the repository root.

You will use three ports on localhost:

- **9000** — your throwaway origin app (what Gate fronts).
- **8444** — Gate's public edge (HTTPS).
- **8445** — Gate's separate admin listener (the Ledger).

## Step 1 — Start a throwaway origin on 127.0.0.1:9000

Gate needs an origin to sit in front of. Any app that returns HTML will do. For this tutorial, serve a tiny static page.

First, create a page to serve:

```
mkdir -p origin-demo && printf '<!doctype html><title>My demo app</title><h1>Hello from my origin</h1>' > origin-demo/index.html
```

Now start a static server bound to loopback, on port 9000:

```
python -m http.server 9000 --bind 127.0.0.1 --directory origin-demo
```

You should see:

```
Serving HTTP on 127.0.0.1 port 9000 (http://127.0.0.1:9000/) ...
```

Leave this running and open a new shell for the next steps.

> **Tip:** This is a plain, unprotected origin — exactly the app you are about to place Gate in front of. In a real deployment, Gate terminates TLS at the edge and forwards to your origin over the private network; the origin itself is not exposed to the internet.

## Step 2 — Build the binary

From the repository root, build the Gate binary:

```
go build -o bin/gate.exe ./cmd/gate
```

When it finishes, the command returns with no output and `bin/gate.exe` exists. That is success — `go build` is quiet when it works.

## Step 3 — Run Gate in global monitor mode

Now start Gate. The `-monitor` flag is the important one: it puts the whole fleet in **global monitor mode**, which downgrades every route to monitor. Gate will score and log, but enforce nothing, everywhere.

```
bin/gate.exe -addr :8444 -upstream http://127.0.0.1:9000 -monitor
```

You should see startup log lines like these:

```
humanymous Gate on https://localhost:8444 -> http://127.0.0.1:9000 (monitor=true)
humanymous Gate admin console on https://localhost:8445/__hmn/admin/console
  dev tokens — auditor:<hex> operator:<hex> approver:<hex> dpo:<hex>
```

A few things to notice:

- `(monitor=true)` confirms enforcement is off across the board. Every verdict is still computed and written to the audit log — it just never changes what the visitor receives.
- The **admin console** is on a *separate* listener (`:8445`), cross-origin to the public edge. This is by design: the admin plane is not reachable from the public edge.
- The `dev tokens` line prints four development admin tokens, one per role (**Auditor**, **Operator**, **Approver**, **DPO**). These are generated fresh each boot unless you set `HMN_ADMIN_TOKENS`. Keep this terminal visible — you may want the operator token shortly.

Leave Gate running.

## Step 4 — Load your app through Gate

Open your browser to:

```
https://localhost:8444
```

**Expect a certificate warning.** The dev certificate is self-signed in memory (CN `humanymous.local`, valid for `localhost`, `humanymous.local`, `127.0.0.1`, `::1`). This is expected in the reference build — production deployments use ACME or a bring-your-own certificate. Accept the warning to proceed (in most browsers: *Advanced* → *proceed to localhost*).

You should now see your origin's page — the "Hello from my origin" heading you created in Step 1. Gate fronted the request, terminated TLS, forwarded to `127.0.0.1:9000`, and streamed the response back.

Here is the interesting part: the HTML that came back is *not* byte-for-byte your origin's HTML. Gate injected the detection bundle into it. To confirm this, view the page source (or fetch it directly) and look for a reference to the control path `/__hmn/` — the injected bundle loads via the control plane, for example `/__hmn/loader.js`.

You can verify from the shell too (the `-k` flag accepts the self-signed dev cert):

```
curl -k https://localhost:8444/
```

Look in the returned HTML for your original `<h1>Hello from my origin</h1>` **and** the injected reference to `/__hmn/`. Seeing both is the proof: your app's content is intact, and Gate's detection bundle is now riding along with it.

The exact markup Gate inserts once at the first `<head>` boundary is:

```
<!--hmn-injected--><script src="/__hmn/loader.js" defer></script>
```

The `<!--hmn-injected-->` comment is an idempotency marker — if it is already present, Gate passes the page through untouched, so injection never happens twice.

## Step 5 — Open the Ledger and watch a verdict appear

Open the Ledger:

```
https://localhost:8445/__hmn/admin/console
```

You will get the same self-signed certificate warning — accept it. In dev, the console is injected with the **operator** token, so it can read and request without you pasting anything.

Go to the **Overview** view ("live edge decisions"). As you reload `https://localhost:8444` in your other browser tab, watch a verdict flow in for each request. Every request Gate handled — including your own page loads — was scored across layers L1–L7 and written to the tamper-evident audit log.

Because you are in monitor mode, note what did **not** happen:

- Nothing was challenged.
- Nothing was blocked.
- Your origin was contacted normally and returned its real content every time.

The verdict on the stream is a *decision that was recorded, not enforced*. That is exactly the point of monitor mode: you get full visibility into what Gate would decide, with no effect on live traffic.

## You did it

In about half an hour you have:

- Stood up an origin, built Gate, and placed it at the edge in front of your app.
- Run the whole fleet in global monitor mode with enforcement off.
- Confirmed your app's HTML came back intact, with the detection bundle injected.
- Watched real verdicts appear on the Overview view — scored and logged, enforced nowhere.

That is the recommended starting posture for any deployment: observe first, trust the audit trail, then decide. Nicely done.

## What next

- **[Will this break my app?](../explanation/will-this-break-my-app.md)** — how enforcement, fail-open behavior, and staged rollout work before you turn anything on.
- **[CLI, Config & Per-Route Policy Reference](../reference/cli-config-policy.md)** — every flag, environment variable, policy preset, and the per-route policy table, for when you move past monitor mode.
