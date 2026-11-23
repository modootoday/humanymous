---
id: plan:root/20261123121500
title: Lane H #4 verdict-token transport and route presets on the served path
status: active
created: 2026-11-23T12:15:00Z
updated: 2026-11-23T12:15:00Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/application.rs, crates/sdk/humanymous-runtime/src/gateway/application/gates.rs, crates/sdk/humanymous-runtime/src/gateway/configuration.rs, crates/sdk/humanymous-runtime/src/gateway/route_policy.rs, crates/sdk/humanymous-runtime/src/gateway/token.rs]
  depends_on: [sot:root/runtime-application, plan:root/20261121201500]
---

# Lane H #4: verdict-token transport and route presets on the served path

## Goal

Owner decision #4 (issue #75 comment 7017558784) asks for Go's key-gated
`hmn_vt` issuer, with route presets and the attestation floor, on the served
path. Steps #2, #3, #5 and #7 of `plan:root/20261121201500` are composed. This
plan records why #4 cannot be ported verbatim, and splits it into the part that
is decidable from canon and the part that needs the owner.

Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
band or signal ID changes. Presets choose enforcement behaviour only. A11y
gating is untouched.

## What exists at base 9d89933d

- Kernels, all Go-golden pinned:
  - `gateway::token` issues and verifies verdict, step-up and receipt tokens, with epochs and the fingerprint binding.
  - `gateway::gates::verdict_token` and `gateway::gates::attestation_floor_audit` are the gate decisions.
  - `gateway::route_policy` holds the presets, path resolution, the routes file and the attested catch-all refusal (owner decision B1).
  - `gateway::settings::configsign` produces the config version.
- Served path:
  - The exact route inventory governs every request.
  - Every inventoried route enforces and none carries the attestation floor.
  - Nothing mints `hmn_vt`, so the upgrade gate denies every upgrade (HR-26), as Go did without a token key.
  - Origin responses with an `Upgrade` field are refused as framing errors, so no upgrade can complete anyway.
- PR #78 (`claude/b12-route-presets`) is superseded: its presets, routes file and config version reached the base as `route_policy` and `configsign`. It is not rebased here.

## The conflict

Go carries both proofs as cookies:
- `hmn_vt`: set on a collect ALLOW, `HttpOnly; Secure; SameSite=Lax`, 30 minutes.
- `hmn_su`: the step-up proof the attestation floor requires.

`sot:root/runtime-application` makes the initial browser profile cookie-free. It
rejects every observed `Cookie` field, clients send `credentials=omit`, and
"route-scoped cookie prepare/identity binding" is named as a mandatory later
contract (`sot:root/rust-migration`). Issuing `hmn_vt` as a cookie therefore
either breaks that contract or is never presented back.

The Rust session is also stronger than Go's `hsid`. Every protected request is
authenticated by request integrity (RIT) under a server-held session key, so the
server already knows the session's last verdict. Go needed a signed client
token for a stateless fast path across nodes.

## Options for the owner

1. **Server-side sticky verdict (recommended).**
   - No `hmn_vt` on the wire. A collect ALLOW records a bounded, expiring verdict on the RIT session (30 minutes, Go's lifetime).
   - The verdict-token gate's fast path reads it: valid ALLOW on an enforcing route → forward without scoring, audited `enforcement.allow`.
   - `gate.verdict_token` monitor/shadow audit `would_fast_path`. A lifted or expired verdict re-scores, matching Go's "clear and re-score".
   - The step-up proof becomes a session flag set by the Pass flow, which the attestation floor reads.
   - No new client credential and no cookie. The key gate becomes "fast path enabled", since no key is needed.
   - Deviation from decision #4's wording: there is no `hmn_vt` token.
2. **Header-carried token.**
   - Issue `hmn_vt` in a response header on collect ALLOW. The client presents it in a request header that joins the signed canonical bytes, which needs a protocol version bump in the shared integrity codec.
   - Keeps the token and the key gate, but changes the collector protocol and the client.
3. **Cookie contract first.**
   - Implement the route-scoped cookie prepare and identity binding contract the SoT names, then issue `hmn_vt` and `hmn_su` as Go did.
   - Largest change. It also lifts the cookie-free restriction for opted-in routes.

## Decidable now (no owner input)

- **Route presets on the served path.** Each inventoried route may name a preset from `route_policy::VALID_PRESETS`; absent means today's behaviour (`balanced`). The preset flags map as follows:
  - `enforce`: `monitor` and `off` act on nothing, as Go does.
  - `fail_closed`: already universal; owner decisions 2026-11-16 and C1 keep `Unknown` challenging.
  - `attest_floor`: `attested`. Refused on the catch-all, as `refuse_attested_catch_all` already does. Inert until the step-up transport is decided.
  - `inject`: kept separate from `inject.routes` (#7), which stays the explicit opt-in.
- The `attested` startup check (`validate_attested_routes`) runs against whichever proof source the owner picks.

## Steps

- [ ] Owner chooses option 1, 2 or 3 (issue #75).
- [ ] Route presets per inventoried route (decidable now): configuration, readiness, served-path `enforce` and monitor/off audit, ledger and SoT.
- [ ] The chosen verdict/step-up transport, with the verdict-token gate composed in its `gate.verdict_token` mode (#3 overlay) and the attestation floor in `gate.attest_floor`.

## Verification

Each step uses scoped `cargo test --locked -p humanymous-runtime`, Clippy and fmt, and Go goldens where Go behaviour is ported. SoT and ledger updates land in the same pull request. Product Docker cases are separate gates.
