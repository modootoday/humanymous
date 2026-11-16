# FM11 class A sweep of legacy Go rows

Evidence for roadmap unit 24 (`plan:root/20261116060213`): whether class A
(autonomous, provable here) port work remains in the legacy Go row ledger
`.agents/retirement/legacy-go-row-review.json`. Baseline is commit 981d25c.

## Method

Every `missing` and `partial` row was reviewed in priority order
(internal/gate, cmd/server, internal/mlcorrect, then the rest). A gap was class
A only if it is pure and deterministic, its owning Rust module already exists,
an accepted SoT or plan authorizes the behavior, and it needs none of: dual
control, Redis/fleet, the admin API/RBAC, an open contract (B02, B11, B12, B26),
a detection-scoring change, a new dependency or Cargo manifest change, audit
keys or Cap/handoff/grant artifacts, or owner acceptance of an intentional
Go/Rust difference. Go behavior was read at 981d25c; the internal/scoring and
internal/audit Go tests were also re-run there with go1.25.3 (pass). The
internal/gate Go package could not be built in the sweep's own scratch extract
(a module outside the frozen go.sum), so its rounds were first ported from
source reading. A separate extract of 981d25c that resolves modules
(`GOTOOLCHAIN=go1.25.3 GOFLAGS=-mod=mod`) builds the package; there, all 90 Go
tests mapped in the four `internal/gate/wargame_escalation_r*_test.go` rows
pass (`go test ./internal/gate -run '^(<mapped names>)$' -count=1`, 90 PASS,
0 FAIL), and the Rust counterparts pass in `cargo test --locked`.

## Class A ported in this sweep

| Commit | Rows | Rust tests |
| --- | --- | --- |
| `test(runtime): port gate wargame kernel rounds R086-R198` | wargame_escalation_r81, r121, r161 (60 Go tests mapped; rows stay partial) | 58 in gateway/token, gateway/ban, gateway/smuggle, network/history/sweep |
| `test(core): port scoring overlay and rule-table resolution tests` | internal/scoring runtime_test, hardrules, resolution_test (partial to ported_unit) | 6 in scoring/engine, scoring/promotion, scoring/consistency |
| `test(runtime): pin empty audit HMAC key refusal` | internal/audit verifier_p0_test (partial to ported_unit); chain.go gap closed | 1 in audit/log |
| `test(runtime): pin non-positive rate hard cutpoint refusal` | internal/gate/settings/settings_test (gap closed; row stays partial) | 1 in network/history/rate |
| `docs(retirement): close the browser-runner gap on detect/bridge.go` | internal/detect/bridge.go (partial to ported_unit) | 2 existing wasm_bindgen_tests, run in real headless Firefox by the hosted firefox-component job 133289839043 (66 passed, 0 ignored) |

Ledger totals: before 52 missing, 101 partial, 159 ported_unit, 2 n/a;
after 52 missing, 96 partial, 164 ported_unit, 2 n/a.

## Conclusion

No class A gap remains among the 148 missing or partial rows below. Each
remaining row is blocked by one category; where a row has several gaps, the
category is its dominant blocker and the row note in the ledger lists the rest.

| Category | Rows | Meaning |
| --- | ---: | --- |
| COMP | 40 | Runtime/Gate/Core composition is unallocated (plan 20261107205817 B-rows marked M; the public-role profile needs Root canonization) |
| ADMIN | 24 | Admin API, RBAC or settings workflow (B19-B21) |
| OPEN | 20 | Open contract choice (B02 certificates, B11 lab role, B12 routes, B26 PROXYv2, config-version hash) |
| SOT | 17 | The owning SoT slice is not accepted (training gate/shells, watermark PNG/trace, resource tiering, audit taxonomy) |
| REDIS | 14 | Redis/ClickHouse fleet or projections (B23/B24) |
| DIFF | 10 | Intentional Go/Rust difference that needs owner acceptance |
| FREEZE | 9 | Detection freeze or a11y (signal emission, scoring overlays, motor timing) |
| DEP | 9 | New dependency or Cargo manifest change (Ed25519 in core, fuzzing, TLS/H2 attack clients) |
| DUAL | 4 | Dual control (roadmap unit 19) |
| KEYS | 1 | Audit keys |

## Remaining rows

| Row | Disposition | Class | Category | Blocker | Citation |
| --- | --- | --- | --- | --- | --- |
| `internal/gate/admin.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/admin_integrity_empty_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/admin_offline_verify_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/admin_settings.go` | partial | not A | ADMIN | settings proposal/class/dry-run/rollback workflow absent; Rust settings are resource-only by design | plan 20261107205817 B20 (M) |
| `internal/gate/admin_settings_test.go` | partial | not A | ADMIN | settings proposal/class/dry-run/rollback workflow absent; Rust settings are resource-only by design | plan 20261107205817 B20 (M) |
| `internal/gate/admin_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/adminauth.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/approval.go` | partial | not A | DUAL | pending-action store, HMAC ticket and kill/erasure effects are dual-control work owned by Root and two human controllers | plan 20261116060213 unit 19; plan 20261107205817 B20 |
| `internal/gate/attestfloor_test.go` | partial | not A | COMP | token issuance/acceptance, step-up endpoint and attested floor are trust upgrades in an unallocated composition | plan 20261107205817 B16/B17 (M); trust upgrades owner-approved |
| `internal/gate/ban.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/ban_integration_test.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/config.go` | partial | not A | OPEN | longest-prefix preset routing is an explicit contract choice not yet made | plan 20261107205817 B12 (C open contract) |
| `internal/gate/configsign.go` | missing | not A | OPEN | keyed effective-policy hash has no accepted contract; Rust config_version is the provisioned scope revision | plan 20261107205817 B09/B19 + config-version contract |
| `internal/gate/configsign_test.go` | missing | not A | OPEN | keyed effective-policy hash has no accepted contract; Rust config_version is the provisioned scope revision | plan 20261107205817 B09/B19 + config-version contract |
| `internal/gate/console.go` | missing | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/console_test.go` | missing | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/control.go` | partial | not A | COMP | token issuance/acceptance, step-up endpoint and attested floor are trust upgrades in an unallocated composition | plan 20261107205817 B16/B17 (M); trust upgrades owner-approved |
| `internal/gate/control_collect.go` | partial | not A | COMP | token issuance/acceptance, step-up endpoint and attested floor are trust upgrades in an unallocated composition | plan 20261107205817 B16/B17 (M); trust upgrades owner-approved |
| `internal/gate/enforce.go` | partial | not A | DIFF | fail-open Unknown on balanced GET conflicts with fail-closed enforcement (AGENTS.md hard constraint 2); owner decision | row note: legacy action() passes Unknown on safe routes |
| `internal/gate/erasure.go` | partial | not A | DUAL | pending-action store, HMAC ticket and kill/erasure effects are dual-control work owned by Root and two human controllers | plan 20261116060213 unit 19; plan 20261107205817 B20 |
| `internal/gate/forward_fidelity_test.go` | partial | not A | COMP | origin-cloak attach, spoof-header denial and forwarding hygiene need the approved origin-auth contract | plan 20261107205817 B14 (M): Root must choose origin-auth wire |
| `internal/gate/gate.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/gates.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/gc_test.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/guard.go` | partial | not A | COMP | origin-cloak attach, spoof-header denial and forwarding hygiene need the approved origin-auth contract | plan 20261107205817 B14 (M): Root must choose origin-auth wire |
| `internal/gate/incident.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/inject.go` | missing | not A | COMP | HTML loader injection has no owning Rust module | plan 20261107205817 B15 (M) |
| `internal/gate/inject_test.go` | missing | not A | COMP | HTML loader injection has no owning Rust module | plan 20261107205817 B15 (M) |
| `internal/gate/integration_test.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/killswitch_test.go` | partial | not A | DUAL | pending-action store, HMAC ticket and kill/erasure effects are dual-control work owned by Root and two human controllers | plan 20261116060213 unit 19; plan 20261107205817 B20 |
| `internal/gate/prodhardening_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/proxyproto.go` | partial | not A | OPEN | PROXYv2 listener wiring needs the public-role profile | plan 20261107205817 B26 + B01 |
| `internal/gate/recon.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/redisledger.go` | missing | not A | REDIS | no owner-qualified fleet coordinator; Redis client would also be a new dependency | plan 20261107205817 B23 (M) |
| `internal/gate/redisledger_test.go` | missing | not A | REDIS | no owner-qualified fleet coordinator; Redis client would also be a new dependency | plan 20261107205817 B23 (M) |
| `internal/gate/redisratelimit.go` | missing | not A | REDIS | no owner-qualified fleet coordinator; Redis client would also be a new dependency | plan 20261107205817 B23 (M) |
| `internal/gate/resolve_test.go` | partial | not A | OPEN | longest-prefix preset routing is an explicit contract choice not yet made | plan 20261107205817 B12 (C open contract) |
| `internal/gate/seams.go` | partial | not A | REDIS | no owner-qualified fleet coordinator; Redis client would also be a new dependency | plan 20261107205817 B23 (M) |
| `internal/gate/settings/classify.go` | missing | not A | ADMIN | settings proposal/class/dry-run/rollback workflow absent; Rust settings are resource-only by design | plan 20261107205817 B20 (M) |
| `internal/gate/settings/classify_test.go` | missing | not A | ADMIN | settings proposal/class/dry-run/rollback workflow absent; Rust settings are resource-only by design | plan 20261107205817 B20 (M) |
| `internal/gate/settings/overlay.go` | missing | not A | FREEZE | scoring/route/net overlays would change detection inputs; Rust refuses detection fields structurally | plan 20261107205817 B20; rust-migration SoT parity rule 1 (freeze) |
| `internal/gate/settings/resolve.go` | missing | not A | FREEZE | scoring/route/net overlays would change detection inputs; Rust refuses detection fields structurally | plan 20261107205817 B20; rust-migration SoT parity rule 1 (freeze) |
| `internal/gate/settings/settings_test.go` | partial | A ported, rest not A | FREEZE | scoring/route/net overlays would change detection inputs; Rust refuses detection fields structurally | plan 20261107205817 B20; rust-migration SoT parity rule 1 (freeze) |
| `internal/gate/settings/store_file.go` | partial | not A | ADMIN | settings proposal/class/dry-run/rollback workflow absent; Rust settings are resource-only by design | plan 20261107205817 B20 (M) |
| `internal/gate/settings/validate.go` | partial | not A | FREEZE | scoring/route/net overlays would change detection inputs; Rust refuses detection fields structurally | plan 20261107205817 B20; rust-migration SoT parity rule 1 (freeze) |
| `internal/gate/settings_engine.go` | missing | not A | FREEZE | scoring/route/net overlays would change detection inputs; Rust refuses detection fields structurally | plan 20261107205817 B20; rust-migration SoT parity rule 1 (freeze) |
| `internal/gate/settings_runtime.go` | partial | not A | ADMIN | settings proposal/class/dry-run/rollback workflow absent; Rust settings are resource-only by design | plan 20261107205817 B20 (M) |
| `internal/gate/settings_runtime_test.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/token.go` | partial | not A | COMP | token issuance/acceptance, step-up endpoint and attested floor are trust upgrades in an unallocated composition | plan 20261107205817 B16/B17 (M); trust upgrades owner-approved |
| `internal/gate/upstream_error_test.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/wargame_axis_redis_r241_test.go` | missing | not A | REDIS | no owner-qualified fleet coordinator; Redis client would also be a new dependency | plan 20261107205817 B23 (M) |
| `internal/gate/wargame_axis_shadow_rate_r251_test.go` | partial | not A | REDIS | no owner-qualified fleet coordinator; Redis client would also be a new dependency | plan 20261107205817 B23 (M) |
| `internal/gate/wargame_escalation_r121_test.go` | partial | A ported, rest not A | ADMIN | remaining rounds are admin RBAC, erasure/kill/bulk-ban HTTP and witness key projection | plan 20261107205817 B19/B20/B21; plan 20261116060213 unit 19 |
| `internal/gate/wargame_escalation_r161_test.go` | partial | A ported, rest not A | ADMIN | remaining rounds are pending/commit ban effects, approval tickets, route presets and admin HTTP | plan 20261107205817 B18/B19/B20/B12; plan 20261116060213 unit 19 |
| `internal/gate/wargame_escalation_r51_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/wargame_escalation_r81_test.go` | partial | A ported, rest not A | ADMIN | remaining rounds are admin HTTP, route preset (C) or inject (M) | plan 20261107205817 B12/B15/B19 (rows partly ported here) |
| `internal/gate/wargame_r01_ban_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/wargame_t4_promotion_test.go` | missing | not A | COMP | token issuance/acceptance, step-up endpoint and attested floor are trust upgrades in an unallocated composition | plan 20261107205817 B16/B17 (M); trust upgrades owner-approved |
| `internal/gate/ws12_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/ws3_test.go` | missing | not A | DUAL | pending-action store, HMAC ticket and kill/erasure effects are dual-control work owned by Root and two human controllers | plan 20261116060213 unit 19; plan 20261107205817 B20 |
| `internal/gate/ws4_test.go` | partial | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `internal/gate/ws5_test.go` | partial | not A | DIFF | fail-open Unknown on balanced GET conflicts with fail-closed enforcement (AGENTS.md hard constraint 2); owner decision | row note: legacy action() passes Unknown on safe routes |
| `internal/gate/ws7_test.go` | partial | not A | COMP | ordered Gate composition (ban/spoof/token/upgrade/sweep gates, HTTP 403 and audit emission) is unallocated | plan 20261107205817 B13 (M) and new-profile canonization by Root |
| `internal/gate/ws9_test.go` | missing | not A | ADMIN | authenticated admin TCP RBAC endpoints absent; Root/Authority must approve credentials/RBAC first | plan 20261107205817 B19 (M) + B21 |
| `cmd/server/acme.go` | missing | not A | OPEN | certificate loading/ACME/self-signed selection has no accepted provider contract; ACME also needs a new dependency | plan 20261107205817 B02 (C): certificate provider contract |
| `cmd/server/acme_test.go` | missing | not A | OPEN | certificate loading/ACME/self-signed selection has no accepted provider contract; ACME also needs a new dependency | plan 20261107205817 B02 (C): certificate provider contract |
| `cmd/server/cert.go` | missing | not A | OPEN | certificate loading/ACME/self-signed selection has no accepted provider contract; ACME also needs a new dependency | plan 20261107205817 B02 (C): certificate provider contract |
| `cmd/server/collect_pipeline.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/counters_test.go` | missing | not A | COMP | operator explain/counters/healthz route and auth are unallocated | plan 20261107205817 B09 (M) |
| `cmd/server/demo_test.go` | missing | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/h1capture.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/handlers.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/headerinfo.go` | partial | not A | OPEN | trusted-proxy client IP derivation is replaced by a transport allowlist; datacenter dataset has no SoT source | plan 20261107205817 B26: no HTTP forwarded-header identity |
| `cmd/server/headerinfo_test.go` | partial | not A | OPEN | trusted-proxy client IP derivation is replaced by a transport allowlist; datacenter dataset has no SoT source | plan 20261107205817 B26: no HTTP forwarded-header identity |
| `cmd/server/helpers.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/launch.go` | missing | not A | OPEN | playground/launch lab replacement has no allocated owner or module | plan 20261107205817 B11 (lab-only role) |
| `cmd/server/launch_parity_test.go` | missing | not A | OPEN | playground/launch lab replacement has no allocated owner or module | plan 20261107205817 B11 (lab-only role) |
| `cmd/server/main.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/ml_setup.go` | partial | not A | DEP | Ed25519 bundle signature needs a core Cargo manifest change | model-training-correction SoT line 65; ed25519-dalek is not a humanymous-core dependency |
| `cmd/server/ml_setup_test.go` | missing | not A | DEP | Ed25519 bundle signature needs a core Cargo manifest change | model-training-correction SoT line 65; ed25519-dalek is not a humanymous-core dependency |
| `cmd/server/mlcorrect_ops.go` | partial | not A | COMP | operator explain/counters/healthz route and auth are unallocated | plan 20261107205817 B09 (M) |
| `cmd/server/mlcorrect_ops_test.go` | partial | not A | COMP | operator explain/counters/healthz route and auth are unallocated | plan 20261107205817 B09 (M) |
| `cmd/server/ops.go` | partial | not A | COMP | operator explain/counters/healthz route and auth are unallocated | plan 20261107205817 B09 (M) |
| `cmd/server/ops_test.go` | partial | not A | COMP | operator explain/counters/healthz route and auth are unallocated | plan 20261107205817 B09 (M) |
| `cmd/server/pass_handler.go` | partial | not A | COMP | Pass handlers, single-use instance state and accessible UI are unallocated | plan 20261107205817 B08 (M) |
| `cmd/server/pass_handler_test.go` | partial | not A | FREEZE | pass velocity/behavior timing emission changes verdicts or gates Pass on motor timing | rust-migration SoT parity rule 5 (a11y) + plan 20261107205817 B08 |
| `cmd/server/pass_proof.go` | partial | not A | FREEZE | pass velocity/behavior timing emission changes verdicts or gates Pass on motor timing | rust-migration SoT parity rule 5 (a11y) + plan 20261107205817 B08 |
| `cmd/server/pass_scoring.go` | missing | not A | FREEZE | pass velocity/behavior timing emission changes verdicts or gates Pass on motor timing | rust-migration SoT parity rule 5 (a11y) + plan 20261107205817 B08 |
| `cmd/server/pass_wargame_axis_test.go` | partial | not A | FREEZE | pass velocity/behavior timing emission changes verdicts or gates Pass on motor timing | rust-migration SoT parity rule 5 (a11y) + plan 20261107205817 B08 |
| `cmd/server/playground.go` | missing | not A | OPEN | playground/launch lab replacement has no allocated owner or module | plan 20261107205817 B11 (lab-only role) |
| `cmd/server/playground_test.go` | missing | not A | OPEN | playground/launch lab replacement has no allocated owner or module | plan 20261107205817 B11 (lab-only role) |
| `cmd/server/powhandler.go` | partial | not A | COMP | Pass handlers, single-use instance state and accessible UI are unallocated | plan 20261107205817 B08 (M) |
| `cmd/server/resource_handlers.go` | partial | not A | COMP | resource/trace/traffic handlers need scoped authorized storage | plan 20261107205817 B07 (M) |
| `cmd/server/rit.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/server.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `cmd/server/sot31_test.go` | partial | not A | DIFF | address-keyed registry is structurally absent in Rust; owner must accept the difference | row note (SoT-31) |
| `cmd/server/tlsconn.go` | partial | not A | COMP | Core demo role, public listener and collect composition are unallocated | plan 20261107205817 B01/B05/B06 (M) |
| `internal/mlcorrect/bundle.go` | partial | not A | DEP | Ed25519 bundle signature needs a core Cargo manifest change | model-training-correction SoT line 65; ed25519-dalek is not a humanymous-core dependency |
| `internal/mlcorrect/bundle_test.go` | partial | not A | DEP | Ed25519 bundle signature needs a core Cargo manifest change | model-training-correction SoT line 65; ed25519-dalek is not a humanymous-core dependency |
| `cmd/gate/main.go` | partial | not A | COMP | public edge listener, flag surface and Gate composition are unallocated | plan 20261107205817 B01-B27 (M/C) |
| `cmd/gate/routes.go` | partial | not A | OPEN | preset routes file and attested catch-all refusal need the route contract | plan 20261107205817 B12 (C) |
| `cmd/gate/routes_test.go` | missing | not A | OPEN | preset routes file and attested catch-all refusal need the route contract | plan 20261107205817 B12 (C) |
| `cmd/gate/tls.go` | missing | not A | OPEN | certificate loading/ACME/self-signed selection has no accepted provider contract; ACME also needs a new dependency | plan 20261107205817 B02 (C): certificate provider contract |
| `cmd/gate/tls_test.go` | missing | not A | OPEN | certificate loading/ACME/self-signed selection has no accepted provider contract; ACME also needs a new dependency | plan 20261107205817 B02 (C): certificate provider contract |
| `cmd/ml-train/main.go` | partial | not A | SOT | cohort/bot-support, tie and anchor-leakage policy and the command-shell slice are not accepted yet | model-training-correction SoT lines 172-178 and 264-268 |
| `cmd/mlcorrect-train/gate.go` | missing | not A | SOT | cohort/bot-support, tie and anchor-leakage policy and the command-shell slice are not accepted yet | model-training-correction SoT lines 172-178 and 264-268 |
| `cmd/mlcorrect-train/gate_test.go` | missing | not A | SOT | cohort/bot-support, tie and anchor-leakage policy and the command-shell slice are not accepted yet | model-training-correction SoT lines 172-178 and 264-268 |
| `cmd/mlcorrect-train/main.go` | missing | not A | SOT | cohort/bot-support, tie and anchor-leakage policy and the command-shell slice are not accepted yet | model-training-correction SoT lines 172-178 and 264-268 |
| `cmd/redteam/attacks.go` | partial | not A | DEP | TLS parrot, raw H2 and proxy/PoW attack families need new client dependencies | no .spec SoT names cmd/redteam or tlsparrot; Cargo.lock has no h2 or uTLS-style client crate |
| `cmd/redteam/h2raw.go` | missing | not A | DEP | TLS parrot, raw H2 and proxy/PoW attack families need new client dependencies | no .spec SoT names cmd/redteam or tlsparrot; Cargo.lock has no h2 or uTLS-style client crate |
| `cmd/redteam/main.go` | partial | not A | DEP | TLS parrot, raw H2 and proxy/PoW attack families need new client dependencies | no .spec SoT names cmd/redteam or tlsparrot; Cargo.lock has no h2 or uTLS-style client crate |
| `cmd/tlsparrot/main.go` | missing | not A | DEP | TLS parrot, raw H2 and proxy/PoW attack families need new client dependencies | no .spec SoT names cmd/redteam or tlsparrot; Cargo.lock has no h2 or uTLS-style client crate |
| `internal/audit/chain.go` | partial | A ported, rest not A | DIFF | in-memory ring/Recent(n) and WAL-less Log are a deliberate design difference | row note: Rust drops the memory ring on purpose |
| `internal/audit/noderegistry.go` | partial | not A | KEYS | per-node Ed25519 key registry touches audit keys | audit-key-custody SoT; task rule (no audit-key changes) |
| `internal/audit/record.go` | partial | not A | SOT | Event* constants without consumers would be folklore (AGENTS.md hard constraint 5) | audit record taxonomy has no accepted Rust event list |
| `internal/audit/sink.go` | partial | not A | DIFF | a separate no-action Emit helper is deliberately not provided | row note: emit_and_act only |
| `internal/audit/sink_adapters_test.go` | missing | not A | REDIS | Redis/ClickHouse audit projections and RecordSink seam need an audit-owned projection contract | plan 20261107205817 B24 (M) |
| `internal/audit/sink_clickhouse.go` | missing | not A | REDIS | Redis/ClickHouse audit projections and RecordSink seam need an audit-owned projection contract | plan 20261107205817 B24 (M) |
| `internal/audit/sink_record.go` | partial | not A | REDIS | Redis/ClickHouse audit projections and RecordSink seam need an audit-owned projection contract | plan 20261107205817 B24 (M) |
| `internal/audit/sink_redis.go` | missing | not A | REDIS | Redis/ClickHouse audit projections and RecordSink seam need an audit-owned projection contract | plan 20261107205817 B24 (M) |
| `internal/audit/verifier.go` | partial | not A | ADMIN | SelfVerify on a live log belongs to the admin integrity endpoint | plan 20261107205817 B19 (M) |
| `internal/audit/wal_test.go` | partial | not A | DIFF | in-memory ring/Recent(n) and WAL-less Log are a deliberate design difference | row note: Rust drops the memory ring on purpose |
| `internal/collector/collector_test.go` | partial | not A | COMP | first-network pinning, labels, RIT/PoW session state belong to the unallocated Core session composition | plan 20261107205817 B06 (P/C) |
| `internal/collector/merge.go` | partial | not A | COMP | first-network pinning, labels, RIT/PoW session state belong to the unallocated Core session composition | plan 20261107205817 B06 (P/C) |
| `internal/collector/session.go` | partial | not A | COMP | first-network pinning, labels, RIT/PoW session state belong to the unallocated Core session composition | plan 20261107205817 B06 (P/C) |
| `internal/correlation/correlation.go` | partial | not A | REDIS | Redis EVAL coordinator absent; only the AtomicCoordinator seam exists | plan 20261107205817 B23 (M) |
| `internal/correlation/correlation_shared_test.go` | partial | not A | REDIS | Redis EVAL coordinator absent; only the AtomicCoordinator seam exists | plan 20261107205817 B23 (M) |
| `internal/livefeed/hub.go` | missing | not A | OPEN | playground/launch lab replacement has no allocated owner or module | plan 20261107205817 B11 (lab-only role) |
| `internal/livefeed/hub_test.go` | missing | not A | OPEN | playground/launch lab replacement has no allocated owner or module | plan 20261107205817 B11 (lab-only role) |
| `internal/logger/handler.go` | partial | not A | DIFF | synchronous emission instead of a bounded async queue | gateway/logging.rs documented difference |
| `internal/logger/logger.go` | partial | not A | DIFF | synchronous emission instead of a bounded async queue | gateway/logging.rs documented difference |
| `internal/logger/logger_test.go` | partial | not A | DIFF | synchronous emission instead of a bounded async queue | gateway/logging.rs documented difference |
| `internal/logger/sink_test.go` | partial | not A | DIFF | synchronous emission instead of a bounded async queue | gateway/logging.rs documented difference |
| `internal/network/fuzz_test.go` | partial | not A | DEP | coverage-guided fuzzing needs a new dev-dependency | no cargo-fuzz/proptest in Cargo.lock |
| `internal/redis/client.go` | missing | not A | REDIS | RESP client would be a new fleet client with no owner | plan 20261107205817 B23 (M) |
| `internal/redis/client_test.go` | missing | not A | REDIS | RESP client would be a new fleet client with no owner | plan 20261107205817 B23 (M) |
| `internal/resource/classify.go` | missing | not A | SOT | tier gating has no accepted SoT, gates heavy media on liveness (a11y question) and emits l5.media/l5.embed signals (freeze) | plan 20261107205817 B07; legacy SoT-10 unavailable |
| `internal/resource/embed.go` | missing | not A | SOT | tier gating has no accepted SoT, gates heavy media on liveness (a11y question) and emits l5.media/l5.embed signals (freeze) | plan 20261107205817 B07; legacy SoT-10 unavailable |
| `internal/resource/gate.go` | missing | not A | SOT | tier gating has no accepted SoT, gates heavy media on liveness (a11y question) and emits l5.media/l5.embed signals (freeze) | plan 20261107205817 B07; legacy SoT-10 unavailable |
| `internal/resource/media.go` | missing | not A | SOT | tier gating has no accepted SoT, gates heavy media on liveness (a11y question) and emits l5.media/l5.embed signals (freeze) | plan 20261107205817 B07; legacy SoT-10 unavailable |
| `internal/resource/resource_test.go` | partial | not A | SOT | tier gating has no accepted SoT, gates heavy media on liveness (a11y question) and emits l5.media/l5.embed signals (freeze) | plan 20261107205817 B07; legacy SoT-10 unavailable |
| `internal/watermark/codec_lsb.go` | missing | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |
| `internal/watermark/codec_png_multi.go` | partial | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |
| `internal/watermark/extract.go` | missing | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |
| `internal/watermark/inject.go` | missing | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |
| `internal/watermark/ledger.go` | partial | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |
| `internal/watermark/lsb_test.go` | missing | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |
| `internal/watermark/watermark_test.go` | partial | not A | SOT | PNG LSB/multi and apply/recover/trace wait for PNG allocation/alpha qualification and runtime-owned trace auth | plan 20261107135500 lines 150-163; resource-watermark SoT section 6 |

## Citation check

Each remaining row's citation was checked against its source document, not
taken from the sweep table alone:

- 116 rows cite `plan:root/20261107205817` B-rows. 104 of those B-rows carry an
  `M` (missing composition) or `C` (contract choice) disposition. The other 12
  cite B11 (lab-only playground and live feed), B24 (Redis/ClickHouse audit
  projections) or B26 (PROXYv2). The plan header covers all of them: "DESIGN
  ONLY ... no source/import/path/Cargo change ... Root must canonize the
  explicit new public contracts below".
- DEP: `Cargo.lock` has no `cargo-fuzz`, `libfuzzer-sys`, `proptest` or
  `arbitrary`. `ed25519-dalek` is locked for another crate but is not a
  `humanymous-core` dependency, and AGENTS.md reserves Cargo manifest changes
  for root integration.
- SOT: `sot:root/model-training-correction` is `status: draft` and lists the
  gate as needs-work. The active resource-watermark SoT section 6 grants only
  payload/carrier and ledger: "Design acceptance is distinct from execution
  allocation".
- DIFF, FREEZE, DUAL and KEYS follow the goal contract: an intended Go/Rust
  difference goes to the owner, detection scoring is frozen, dual control
  needs two human controllers, and audit keys are not touched.

Coverage: the "Remaining rows" table names exactly the 148 ledger rows whose
disposition is `missing` or `partial`, each once (set comparison of
`baseline_path` against the table's first column: equal, 148 = 148), and every
row carries a class, a category and a citation. No open row is unclassified.
