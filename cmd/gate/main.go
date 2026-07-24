// SPDX-License-Identifier: Apache-2.0

// Command gate is the humanymous Gate reverse-proxy security layer (SoT-19..28,31,32):
// (Browser) -> (humanymous Gate: TLS terminate + streaming HTML injection +
// L1-L7 scoring + edge enforcement + tamper-evident audit) -> (origin app).
//
// It fronts an existing upstream the operator does not control, injects the
// detection bundle into HTML on the fly, enforces the L7 verdict at the edge,
// and emits every decision into the audit chain before it takes effect.
package main

import (
	"context"
	"crypto/ecdsa"
	"crypto/ed25519"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/hex"
	"flag"
	"log"
	"math/big"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"golang.org/x/net/http2"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/gate"
	"github.com/modootoday/humanymous/internal/redis"
	"github.com/modootoday/humanymous/internal/scoring"
)

// version is stamped at release via -ldflags "-X main.version=<tag>-<sha>-<date>"
// so a running Gate self-reports which build it is (audit LOW-4).
var version = "dev"

func main() {
	addr := flag.String("addr", ":8444", "public edge listen address")
	adminAddr := flag.String("admin-addr", "127.0.0.1:8445", "SEPARATE admin listener (auth-gated; SoT-28 WS1). Defaults to LOOPBACK — front it with mTLS/SSO before exposing off-host (audit SEC-1).")
	upstream := flag.String("upstream", "http://127.0.0.1:9000", "origin upstream base URL")
	node := flag.String("node", "gate-1", "node id (audit chain owner)")
	monitor := flag.Bool("monitor", false, "global monitor/shadow mode (score+log, enforce nothing)")
	originKeyHex := flag.String("origin-key", "", "origin-cloaking HMAC key (hex); origin validates X-Hmny-Origin-Auth")
	keystorePath := flag.String("keystore", "", "sealed keystore path for persistent node identity (SoT-28 WS8; needs HMN_UNSEAL); empty = ephemeral")
	// SoT-31 R1 — edge TLS: bring-your-own cert or Let's Encrypt (autocert). None => self-signed.
	tlsCert := flag.String("tls-cert", "", "edge TLS certificate file (PEM); pair with -tls-key for bring-your-own TLS")
	tlsKey := flag.String("tls-key", "", "edge TLS private key file (PEM)")
	acmeDomain := flag.String("acme-domain", "", "comma-separated domain(s) for a Let's Encrypt edge cert via TLS-ALPN-01 (requires binding :443)")
	acmeCache := flag.String("acme-cache", "acme-cache", "directory to cache issued ACME certificates")
	acmeEmail := flag.String("acme-email", "", "optional ACME account contact email")
	acmeDirectory := flag.String("acme-directory", "", "ACME directory URL; empty = production Let's Encrypt. Set to the LE staging URL (https://acme-staging-v02.api.letsencrypt.org/directory) to dry-run issuance without hitting production rate limits, or to ZeroSSL / an internal step-ca")
	// SoT-31 R2 — external route table (prefix -> preset); empty uses the built-in presets.
	routesFile := flag.String("routes", "", "path to a route policy file (`<prefix> <preset>` per line); empty = built-in presets")
	// SoT-32 — durable audit WAL: the tamper-evident chain survives restarts and the
	// in-memory window is bounded. Empty = ephemeral in-memory (dev, unchanged).
	auditWAL := flag.String("audit-wal", "", "durable audit WAL directory (SoT-32); empty = ephemeral in-memory")
	auditVerify := flag.Bool("audit-verify", false, "replay the audit WAL, verify the chain, print the result, and exit")
	auditRedis := flag.String("audit-redis", "", "Redis host:port to project the audit stream to (SoT-32 Tier 1 hot); empty = off")
	auditCH := flag.String("audit-clickhouse", "", "ClickHouse HTTP base URL (e.g. http://ch:8123) to project the audit log to (SoT-32 Tier 2 cold); empty = off")
	// PLAN-08 R1 — shared ban + sticky-verdict state across a gate fleet. Empty =
	// single-node in-memory (unchanged). When set, a ban or DENY raised on any node
	// is enforced on all nodes; a Redis outage degrades each node to its local view.
	redisAddr := flag.String("redis", "", "Redis host:port for shared ban + verdict state (PLAN-08 R1); empty = single-node in-memory")
	// PLAN-08 R4 — trusted L4 balancer CIDRs. When set, the public listener reads a
	// PROXY protocol v2 header from these sources ONLY and recovers the real client IP,
	// so the gate can sit behind a TCP-passthrough LB while keeping IP-keyed bans /
	// rate limits / correlation correct. Empty = disabled (direct client IP, unchanged).
	trustedProxies := flag.String("trusted-proxies", "", "comma-separated CIDRs of L4 balancers allowed to send a PROXY v2 header (PLAN-08 R4); empty = disabled")
	// PLAN-08 R3 — Web Bot Auth allowlist: a file of `keyid base64url-ed25519-pubkey`
	// lines. A valid signature from a listed key is a trust-upgrade; a forgery of a
	// listed key is denied. Empty = feature off.
	agentKeysFile := flag.String("agent-keys", "", "path to a Web Bot Auth trusted-key allowlist (PLAN-08 R3); empty = disabled")
	// PLAN-08 R5 — shadow anomaly observer: watch per-fingerprint request inter-arrival
	// through a streaming MAD detector and LOG outliers. Strictly observational (never
	// affects the verdict); off by default. Shadow-first before any signal earns weight.
	anomalyShadow := flag.Bool("anomaly-shadow", false, "enable the log-only shadow anomaly observer (PLAN-08 R5); never affects verdicts")
	// PLAN-08 R2 — Privacy Pass PAT issuer keys: a PEM file of trusted issuer RSA public
	// keys. A request carrying a valid Private Access Token from a listed issuer is
	// trust-upgraded. Empty = feature off.
	patIssuersFile := flag.String("pat-issuers", "", "path to a PEM file of trusted Privacy Pass PAT issuer public keys (PLAN-08 R2); empty = disabled")
	// PLAN-08 R2 — WebAuthn credential allowlist: a file of `credentialId
	// base64url-spki-ecdsa-p256-pubkey` lines. A valid, fresh possession assertion from
	// a listed credential is trust-upgraded. Empty = feature off.
	webauthnCredsFile := flag.String("webauthn-creds", "", "path to a WebAuthn registered-credential allowlist (PLAN-08 R2); empty = disabled")
	// Production ops hardening (prod-delta closure): graceful drain, request-body cap,
	// HSTS, structured logs, and mTLS on the admin plane. All safe defaults preserve the
	// existing single-binary behavior; detection is unchanged.
	shutdownGrace := flag.Duration("shutdown-grace", 25*time.Second, "graceful-shutdown drain timeout on SIGINT/SIGTERM (align with the orchestrator termination grace)")
	maxBody := flag.Int64("max-body", 0, "max request-body size in bytes forwarded to origin on the proxy path (0 = unlimited); large uploads above this get 413")
	hsts := flag.Bool("hsts", false, "add a Strict-Transport-Security header to edge responses (only enable once real certs and HTTPS-everywhere are in place)")
	adminMTLSCA := flag.String("admin-mtls-ca", "", "PEM file of client-cert CA(s); when set, the admin listener REQUIRES a verified client certificate (mTLS) in addition to the bearer token")
	flag.Parse()

	originKey := []byte(*originKeyHex)
	if len(originKey) == 0 {
		originKey = make([]byte, 32)
		mustRand(originKey)
	}

	// Audit log FIRST (SoT-18): nothing enforces until decisions have a home.
	// Key persistence (SoT-28 WS8): with -keystore, the STH signing seed + HMAC
	// key + vault linkage keys are sealed and resumed across restarts, so a reboot
	// keeps the same chain identity and the same pseudonym linkage (no accidental
	// mass crypto-shred). Without it, keys are ephemeral (dev).
	var hmacKey, signingSeed, witnessSeed []byte
	var vault *audit.Vault
	unseal := os.Getenv("HMN_UNSEAL")
	if *keystorePath != "" {
		if unseal == "" {
			log.Fatalf("keystore requires an unseal passphrase in HMN_UNSEAL")
		}
		m, created, err := audit.LoadOrCreateKeys(*keystorePath, unseal)
		if err != nil {
			log.Fatalf("keystore: %v", err)
		}
		hmacKey, signingSeed, witnessSeed, vault = m.HMACKey, m.SigningSeed, m.WitnessSeed, audit.LoadVault(m.Vault)
		if created {
			log.Printf("keystore: created new sealed node identity at %s", *keystorePath)
		} else {
			log.Printf("keystore: resumed persisted node identity from %s", *keystorePath)
		}
	} else {
		hmacKey = make([]byte, 32)
		mustRand(hmacKey)
		vault = audit.NewVault()
	}
	vault.SetStretch(true) // KDF-stretch pseudonyms (SoT-28 WS8 brute-force resistance)
	// Independent witness co-signs each Signed Tree Head (SoT-28 WS8) so the writer cannot
	// rewrite history undetected. With a keystore its key is PERSISTED (witnessSeed) so
	// pre-restart checkpoints keep verifying and fork-detection survives a restart
	// (deep-review); without one it is ephemeral (dev).
	var witness *audit.Witness
	if len(witnessSeed) == ed25519.SeedSize {
		witness = audit.NewWitnessFromSeed(witnessSeed)
	} else {
		witness = audit.NewWitness()
	}
	// SoT-32 durable audit sink: with -audit-wal the chain is fsync'd to disk and
	// replayed on boot; the STH signing key comes from -keystore so replayed
	// checkpoints verify across restarts.
	var auditSink *audit.WALSink
	if *auditWAL != "" {
		ws, werr := audit.NewWALSink(*auditWAL)
		if werr != nil {
			log.Fatalf("audit-wal: %v", werr)
		}
		auditSink = ws
	}
	// SoT-32 Tier 1/2 projections (async, best-effort; never block the seal).
	var projections []audit.RecordSink
	if *auditRedis != "" {
		projections = append(projections, audit.NewRedisStreamSink(*auditRedis, "audit:"+*node, 100000, 8192))
		log.Printf("audit projection: Redis Streams -> %s (audit:%s)", *auditRedis, *node)
	}
	if *auditCH != "" {
		ch := audit.NewCHSink(*auditCH, "audit_log", 10000, time.Second, 100000)
		// PLAN-08 backlog: optional ClickHouse credential via HTTP Basic Auth (kept out of
		// the URL, so it never lands in a logged endpoint string).
		if u := os.Getenv("HMN_CLICKHOUSE_USER"); u != "" {
			ch.SetAuth(u, os.Getenv("HMN_CLICKHOUSE_PASSWORD"))
		}
		projections = append(projections, ch)
		log.Printf("audit projection: ClickHouse -> %s (audit_log)", *auditCH)
	}
	alog := audit.NewLog(audit.Config{NodeID: *node, HMACKey: hmacKey, CheckpointEvery: 32, Witness: witness, SigningSeed: signingSeed, WAL: auditSink, Projections: projections})
	// Restore the witness's monotonic fork-detection state from the last replayed
	// checkpoint so the first post-restart co-sign still demands an append-only
	// consistency proof (deep-review: no cross-restart amnesia).
	if cps := alog.Checkpoints(); len(cps) > 0 {
		last := cps[len(cps)-1]
		witness.Restore(last.TreeSize, last.MerkleRoot, last.Root)
	}
	if *auditVerify {
		res := alog.SelfVerify()
		if res.OK {
			log.Printf("audit-verify: OK (%d records)", alog.Len())
			os.Exit(0)
		}
		log.Fatalf("audit-verify: FAILED class=%s seq=%d %s", res.Class, res.AtSeq, res.Detail)
	}
	sink := audit.NewSink(alog)
	sink.Emit(audit.Record{EventType: audit.EventInstanceStartup, Actor: audit.Actor{Kind: "system"}, TenantID: *node, KeyID: "k1"})

	// Shutdown (SIGINT/SIGTERM) is handled centrally at the end of main: it drains
	// both listeners via http.Server.Shutdown, THEN seals the keystore (WS8), so an
	// in-flight request is never cut and identity is persisted on every clean stop —
	// not only when -keystore is set. See the graceful-shutdown block below.

	// Shared scoring/verdict state (SoT-22 externalizes this to Redis in prod).
	store := collector.NewStore(30 * time.Minute)
	engine := scoring.NewEngine()

	// PLAN-08 R1 — with -redis, ban + sticky-verdict state is shared fleet-wide via a
	// single Redis client; both the control plane (which SETs verdicts on /collect) and
	// the edge Server (which GETs them, and owns the ban ledger) share the SAME ledgers.
	// Empty keeps the single-node in-memory stores, unchanged.
	var verdicts gate.VerdictLedger = gate.NewVerdictStore(30 * time.Minute)
	var sharedBans gate.BanLedger // nil => NewServer builds an in-memory BanStore
	if *redisAddr != "" {
		rc := redis.New(*redisAddr)
		// PLAN-08 backlog — Redis hardening: optional AUTH (HMN_REDIS_PASSWORD /
		// HMN_REDIS_USER) and a fleet-shared HMAC key (HMN_REDIS_KEY) that KEY-BINDS stored
		// verdict/ban values, so a compromised coordinator that can write Redis cannot
		// forge, relocate, or roll back an ALLOW or a ban (it lacks the key; freshness is
		// re-checked on read). It can still WITHHOLD (availability), which degrades to the
		// local view. Values are UNSIGNED unless HMN_REDIS_KEY is set.
		if pw := os.Getenv("HMN_REDIS_PASSWORD"); pw != "" {
			rc.SetAuth(os.Getenv("HMN_REDIS_USER"), pw)
		}
		redisKey := []byte(os.Getenv("HMN_REDIS_KEY"))
		if len(redisKey) == 0 {
			log.Printf("WARNING: -redis set WITHOUT HMN_REDIS_KEY — the shared verdict/ban channel is UNSIGNED; a weak-ACL or on-path Redis can inject ALLOW/DENY/ban state. Set HMN_REDIS_KEY (>=16 bytes, secret) AND HMN_REDIS_PASSWORD, and isolate Redis on a trusted network.")
		} else if len(redisKey) < 16 || string(redisKey) == "fleet-shared-hmac-secret" {
			log.Fatalf("HMN_REDIS_KEY must be a secret of at least 16 bytes (and not a demo/placeholder value)")
		}
		verdicts = gate.NewRedisVerdictLedger(rc, 30*time.Minute, redisKey)
		sharedBans = gate.NewRedisBanLedger(rc, gate.DefaultRateWindow, gate.DefaultRateSoft, gate.DefaultRateHard, redisKey)
		mode := "unsigned"
		if len(redisKey) > 0 {
			mode = "key-bound HMAC values"
		}
		log.Printf("shared state: Redis %s (%s; bans + sticky verdicts propagate fleet-wide, PLAN-08 R1)", *redisAddr, mode)
	}

	// Verdict-token HMAC key. By default it is per-boot random (single node). For a
	// FLEET (or to keep tokens valid across restarts) set HMN_TOKEN_KEY (hex, >=32 chars)
	// so every node signs/verifies the same tokens (deep-review). Without it a token
	// minted on one node/boot simply fails to verify elsewhere and the request falls
	// through to re-scoring (no deny) — correct but it loses the cross-node fast-path.
	tokenKey := make([]byte, 32)
	sharedTokenKey := false
	if tk := os.Getenv("HMN_TOKEN_KEY"); tk != "" {
		b, err := hex.DecodeString(tk)
		if err != nil || len(b) < 16 {
			log.Fatalf("HMN_TOKEN_KEY must be hex of at least 16 bytes")
		}
		tokenKey = b
		sharedTokenKey = true
	} else {
		mustRand(tokenKey)
		if *redisAddr != "" {
			log.Printf("WARNING: -redis (fleet mode) set WITHOUT HMN_TOKEN_KEY — verdict tokens are per-node, so a returning human is re-scored (not fast-pathed) on other nodes. Set a shared HMN_TOKEN_KEY across the fleet.")
		}
	}
	// Shared rotating token epoch (SoT-28 WS6): the control plane mints under the
	// current epoch, the edge accepts current+previous, and a timer rotates.
	epochs := gate.NewEpochManager()
	control := gate.NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(tokenKey).WithTokenEpochs(epochs)
	if *anomalyShadow {
		control.WithCohortShadow() // ceiling-guard #3: population/cohort behavioral shadow (log-only)
	}

	// PLAN-08 R3 — load the Web Bot Auth trusted-key allowlist, if configured.
	var agentKeys gate.KeyDirectory
	if *agentKeysFile != "" {
		raw, err := os.ReadFile(*agentKeysFile)
		if err != nil {
			log.Fatalf("agent-keys: %v", err)
		}
		dir, err := gate.NewStaticKeyDirectory(string(raw))
		if err != nil {
			log.Fatalf("agent-keys: %v", err)
		}
		agentKeys = dir
		log.Printf("Web Bot Auth enabled: verifying agent signatures against %s (PLAN-08 R3)", *agentKeysFile)
	}

	// PLAN-08 R2 — load the Privacy Pass PAT issuer allowlist, if configured.
	var patIssuers *gate.PATVerifier
	if *patIssuersFile != "" {
		raw, err := os.ReadFile(*patIssuersFile)
		if err != nil {
			log.Fatalf("pat-issuers: %v", err)
		}
		pv, err := gate.NewPATVerifier(raw)
		if err != nil {
			log.Fatalf("pat-issuers: %v", err)
		}
		patIssuers = pv
		log.Printf("Privacy Pass enabled: verifying Private Access Tokens against %s (PLAN-08 R2)", *patIssuersFile)
	}

	// PLAN-08 R2 — load the WebAuthn registered-credential allowlist, if configured.
	var webauthnCreds *gate.WebAuthnRegistry
	if *webauthnCredsFile != "" {
		raw, err := os.ReadFile(*webauthnCredsFile)
		if err != nil {
			log.Fatalf("webauthn-creds: %v", err)
		}
		wc, err := gate.NewWebAuthnRegistry(string(raw))
		if err != nil {
			log.Fatalf("webauthn-creds: %v", err)
		}
		// PLAN-08 backlog — optional origin + RP-ID binding so an assertion minted for
		// another site cannot be replayed here (empty = the respective check is off).
		wc.SetBinding(os.Getenv("HMN_WEBAUTHN_ORIGIN"), os.Getenv("HMN_WEBAUTHN_RPID"))
		webauthnCreds = wc
		log.Printf("WebAuthn enabled: verifying possession assertions against %s (PLAN-08 R2)", *webauthnCredsFile)
	}

	cfg := gate.Config{
		Upstream:      *upstream,
		NodeID:        *node,
		ControlPath:   "/__hmn/",
		GlobalMonitor: *monitor,
		OriginKey:     originKey,
		TokenKey:      tokenKey,
		TokenEpochs:   epochs,
		BanLedger:     sharedBans,     // shared Redis ban ledger, or nil for in-memory (PLAN-08 R1)
		AgentKeys:     agentKeys,      // Web Bot Auth directory, or nil (PLAN-08 R3)
		AnomalyShadow: *anomalyShadow, // R5 shadow observer (log-only), off by default
		PATIssuers:    patIssuers,     // Privacy Pass PAT issuers, or nil (PLAN-08 R2)
		WebAuthnCreds: webauthnCreds,  // WebAuthn credential registry, or nil (PLAN-08 R2)
		MaxBodyBytes:  *maxBody,       // proxy-path request-body cap (0 = unlimited)
		HSTS:          *hsts,          // Strict-Transport-Security on edge responses
		Routes: map[string]string{
			"/login":    "strict",
			"/checkout": "strict",
			"/admin":    "strict",
			"/health":   "off",
		},
	}
	// SoT-31 R2 — override the built-in route presets with an operator file so an
	// adopter protects THEIR paths without recompiling.
	if *routesFile != "" {
		r, rErr := loadRoutes(*routesFile)
		if rErr != nil {
			log.Fatalf("routes: %v", rErr)
		}
		cfg.Routes = r
		log.Printf("loaded %d route rule(s) from %s", len(r), *routesFile)
	}

	// Ceiling-guard #1 startup validation (see validateAttestedRoutes): refuse to start if
	// an attested route is configured without a shared HMN_TOKEN_KEY — the Gate's per-boot
	// random key could never verify a Core-minted step-up receipt, so /__hmn/stepup could
	// never mint hmn_su and the route becomes a CHALLENGE→Pass wall NO human can clear
	// (worse than the lockout the top constraint forbids). Warn (non-fatal) when a verifier
	// is absent (the floor then holds as a Pass-for-everyone friction wall).
	hasVerifier := agentKeys != nil || patIssuers != nil || webauthnCreds != nil
	warns, err := validateAttestedRoutes(cfg.Routes, sharedTokenKey, hasVerifier)
	if err != nil {
		log.Fatalf("routes: %v", err)
	}
	for _, wmsg := range warns {
		log.Printf("WARNING: %s", wmsg)
	}

	srv, err := gate.NewServer(cfg, sink, vault, verdicts, control.Handler())
	if err != nil {
		log.Fatalf("proxy: %v", err)
	}

	// Seed admin operators with bearer tokens (SoT-28 WS1/WS2). Production issues
	// these via mTLS client certs or an SSO/operator-CA login; the reference
	// prints dev tokens at startup. The console is injected with the operator
	// token so it can read + request; approvals need a distinct approver token.
	toks := map[gate.Role]string{}
	// Deterministic dev tokens via HMN_ADMIN_TOKENS="auditor:tok,operator:tok,..."
	// (for e2e/demo); otherwise generate random per boot.
	envToks := map[string]string{}
	for _, kv := range strings.Split(os.Getenv("HMN_ADMIN_TOKENS"), ",") {
		if p := strings.SplitN(kv, ":", 2); len(p) == 2 {
			envToks[p[0]] = p[1]
		}
	}
	// SoT-31 R3 + audit SEC-3 — fail-safe defaults: outside the explicit local-demo
	// opt-in, refuse to boot with shipped/placeholder/weak admin secrets, so a
	// `cp .env.example .env && up` cannot go live on CHANGE-ME credentials or a
	// `e2e-*` demo token, and a low-entropy unseal passphrase cannot seal the keystore.
	devTokens := os.Getenv("HMN_ALLOW_DEV_TOKENS") == "1"
	if !devTokens {
		reject := func(kind, v string) {
			lv := strings.ToLower(v)
			if strings.HasPrefix(v, "e2e-") || strings.Contains(lv, "change") ||
				strings.Contains(lv, "placeholder") || strings.Contains(lv, "example") || strings.Contains(lv, "your-") {
				log.Fatalf("refusing to boot: %s is a shipped/placeholder value; set a real secret, or HMN_ALLOW_DEV_TOKENS=1 for the local demo only (SoT-31 R3 / audit SEC-3)", kind)
			}
			if len(v) < 16 {
				log.Fatalf("refusing to boot: %s is too short (<16 chars); use a high-entropy secret, or HMN_ALLOW_DEV_TOKENS=1 for the local demo only (audit SEC-3)", kind)
			}
		}
		for role, v := range envToks {
			reject("HMN_ADMIN_TOKENS["+role+"]", v)
		}
		if unseal != "" {
			reject("HMN_UNSEAL", unseal)
		}
	}
	for _, role := range []gate.Role{gate.RoleAuditor, gate.RoleOperator, gate.RoleApprover, gate.RoleDPO} {
		t := envToks[string(role)]
		if t == "" {
			t = randHex(24)
		}
		toks[role] = t
		srv.Auth().Add(t, string(role)+"-1", role)
	}
	// audit SEC-1: only hand the console a live operator bearer token in the explicit
	// local-demo mode. In a real deployment the console loads WITHOUT an injected token,
	// so admin API calls require a real bearer token through the auth gate — front the
	// admin plane with mTLS/SSO. (This closes the "any TLS client that reaches :8445 is
	// handed a working operator token" bypass.)
	if devTokens {
		srv.SetDevConsoleToken(toks[gate.RoleOperator])
	}

	// SoT-31 R1 — the public edge serves a real cert (BYO / ACME) or self-signed;
	// the admin plane stays self-signed (management-network + mTLS is the real
	// control), so ACME challenges never depend on the admin port.
	edgeCfg, err := buildEdgeTLS(edgeTLS{
		acmeDomains: splitDomains(*acmeDomain), acmeCache: *acmeCache, acmeEmail: *acmeEmail,
		acmeDirectory: *acmeDirectory, certFile: *tlsCert, keyFile: *tlsKey,
	})
	if err != nil {
		log.Fatalf("edge tls: %v", err)
	}
	adminCert, err := selfSignedCert()
	if err != nil {
		log.Fatalf("admin cert: %v", err)
	}
	adminCfg := &tls.Config{Certificates: []tls.Certificate{adminCert}, MinVersion: tls.VersionTLS12}
	// Optional mTLS on the admin plane (prod-delta closure B1): require a client cert
	// signed by an operator-supplied CA, in ADDITION to the bearer token. This is the
	// shippable half of "front the admin plane with mTLS/SSO" — SSO stays a reverse proxy.
	if *adminMTLSCA != "" {
		caPEM, rerr := os.ReadFile(*adminMTLSCA)
		if rerr != nil {
			log.Fatalf("admin-mtls-ca: %v", rerr)
		}
		pool := x509.NewCertPool()
		if !pool.AppendCertsFromPEM(caPEM) {
			log.Fatalf("admin-mtls-ca: no valid certificate found in %s", *adminMTLSCA)
		}
		adminCfg.ClientAuth = tls.RequireAndVerifyClientCert
		adminCfg.ClientCAs = pool
		log.Printf("admin listener: mTLS ENABLED — a client certificate signed by %s is required in addition to the bearer token", *adminMTLSCA)
	}
	mkServer := func(a string, h http.Handler, tc *tls.Config) *http.Server {
		hs := &http.Server{
			Addr: a, Handler: h, TLSConfig: tc,
			ReadHeaderTimeout: 8 * time.Second,  // slowloris (SoT-17/25)
			ReadTimeout:       30 * time.Second, // slow-POST
			WriteTimeout:      30 * time.Second,
			IdleTimeout:       60 * time.Second,
			MaxHeaderBytes:    1 << 20,
		}
		// HTTP/2 DoS caps on BOTH listeners (SoT-25 §3, SoT-28 WS1: the admin
		// listener re-applies the same ingress hardening as the public edge).
		_ = http2.ConfigureServer(hs, &http2.Server{MaxConcurrentStreams: 100, MaxReadFrameSize: 1 << 20, MaxUploadBufferPerConnection: 1 << 20})
		return hs
	}

	// Separate authenticated admin listener (SoT-28 WS1). It is cross-origin to
	// the public edge, so origin-served JS cannot reach it via same-origin fetch.
	// Execute due crypto-shreds (past their hold window) periodically (SoT-28 WS3).
	go func() {
		t := time.NewTicker(10 * time.Second)
		for range t.C {
			srv.RunDueShreds(time.Now())
		}
	}()
	// The verdict-token epoch now rotates automatically by the wall clock (see
	// EpochManager) so every fleet node agrees without coordination — no per-node ticker,
	// which previously desynced the fleet (deep-review). epochs.Advance() remains available
	// as a manual fleet-wide emergency revocation.
	// Sweep in-memory detection state every minute so fingerprint/IP-keyed maps cannot
	// grow without bound under bot-flood churn (PLAN-08 deployment-review ship-blocker;
	// brings the gate to parity with the core engine's GC ticker). The same tick surfaces
	// audit-projection drop counters (PLAN-08 backlog): a Tier-1/Tier-2 sink that is
	// dropping records under backpressure/outage was previously silent.
	var lastDropped uint64
	go func() {
		t := time.NewTicker(time.Minute)
		for range t.C {
			now := time.Now()
			srv.GC(now)     // verdicts, bans + strikes + rate windows, sweep bindings, anomaly, cred fan-out
			store.GC(now)   // control-plane collector store
			control.GC(now) // control-plane observers (cohort shadow)
			var dropped uint64
			for _, p := range projections {
				if d, ok := p.(interface{ Dropped() uint64 }); ok {
					dropped += d.Dropped()
				}
			}
			if dropped > lastDropped {
				log.Printf("WARN audit projection dropped %d records total (Tier-1/2 backpressure or outage; the WAL remains the durability authority)", dropped)
				lastDropped = dropped
			}
		}
	}()

	if lo := strings.HasPrefix(*adminAddr, "127.0.0.1") || strings.HasPrefix(*adminAddr, "localhost") || strings.HasPrefix(*adminAddr, "[::1]"); !lo && !devTokens {
		log.Printf("WARNING (audit SEC-1): admin listener %s is not bound to loopback and no mTLS/SSO is configured — front it with a mutually-authenticated proxy or bind -admin-addr to 127.0.0.1. In Docker, keep the host port mapping loopback-only (127.0.0.1:8445:8445).", *adminAddr)
	}
	adminSrv := mkServer(*adminAddr, srv.AdminHandler(), adminCfg)
	go func() {
		log.Printf("humanymous Gate admin console on https://%s/__hmn/admin/console", *adminAddr)
		// SoT-31 R4 / audit SEC-2: NEVER echo admin bearer-token values at INFO level in a
		// real deployment — env-supplied production tokens would land in stdout / docker logs
		// / log shippers. Print the raw values only in the explicit local-demo mode; otherwise
		// print role names only.
		if devTokens {
			log.Printf("  dev tokens (demo mode) — auditor:%s operator:%s approver:%s dpo:%s", toks[gate.RoleAuditor], toks[gate.RoleOperator], toks[gate.RoleApprover], toks[gate.RoleDPO])
		} else {
			log.Printf("  admin roles configured: auditor, operator, approver, dpo — token values NOT logged; supply them via HMN_ADMIN_TOKENS (a random token was generated for any role left unset)")
		}
		if err := adminSrv.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
			log.Fatalf("admin listener: %v", err)
		}
	}()

	pubSrv := mkServer(*addr, srv, edgeCfg)
	tlsMode := "self-signed"
	if len(splitDomains(*acmeDomain)) > 0 {
		tlsMode = "ACME " + *acmeDomain
	} else if *tlsCert != "" && *tlsKey != "" {
		tlsMode = "BYO cert"
	}
	log.Printf("humanymous Gate %s on https://localhost%s -> %s (monitor=%v, tls=%s)", version, *addr, *upstream, *monitor, tlsMode)
	// Topology honesty (deep-review): the gate does NOT capture the ClientHello, so JA3/JA4,
	// grease, the H2 fingerprint and the UA-vs-TLS/H2 cross-checks (HR-2/HR-5/HR-11/HR-14) do
	// NOT fire here — the network-fingerprint plane is a reference feature of the Core engine.
	// Detection at the gate is client JS/WASM + header/behavior + IP-intel only. Additionally,
	// if a CDN/L7 balancer terminates TLS in front, even the Core's TLS plane is inert.
	log.Printf("NOTE: network-fingerprint plane (JA3/JA4/H2) is INACTIVE at the gate — detection here is JS/WASM + headers + behavior + IP-intel. For TLS fingerprinting, terminate raw TLS at the Core engine with no re-terminating CDN/L7-LB in front.")

	// PLAN-08 R4 — behind an L4 passthrough LB: parse PROXY v2 (below TLS) from the
	// trusted balancer CIDRs, recovering the real client IP. Without trusted proxies,
	// keep the standard ListenAndServeTLS path unchanged.
	// Start the edge listener in the background so main can block on a shutdown signal.
	go func() {
		var err error
		if *trustedProxies != "" {
			cidrs, perr := gate.ParseCIDRs(*trustedProxies)
			if perr != nil {
				log.Fatalf("trusted-proxies: %v", perr)
			}
			base, lerr := net.Listen("tcp", *addr)
			if lerr != nil {
				log.Fatalf("edge listener: %v", lerr)
			}
			log.Printf("  PROXY protocol v2 enabled for %d trusted CIDR(s) (real client IP recovered behind L4 passthrough)", len(cidrs))
			// PROXY header is read below TLS; then tls.NewListener terminates the handshake.
			tln := tls.NewListener(gate.WrapProxyProto(base, cidrs), edgeCfg)
			err = pubSrv.Serve(tln)
		} else {
			err = pubSrv.ListenAndServeTLS("", "")
		}
		if err != nil && err != http.ErrServerClosed {
			log.Fatalf("edge listener: %v", err)
		}
	}()

	// Graceful shutdown: on SIGINT/SIGTERM drain both listeners (bounded by -shutdown-grace)
	// so in-flight requests complete, THEN seal the keystore so identity persists on every
	// clean stop. A second signal restores default handling and force-quits.
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	<-ctx.Done()
	stop()
	control.SetReady(false) // /__hmn/readyz now 503s so a load balancer drains this node first
	log.Printf("shutdown: signal received; readiness now draining; draining connections (grace %s)...", *shutdownGrace)
	dctx, cancel := context.WithTimeout(context.Background(), *shutdownGrace)
	defer cancel()
	_ = pubSrv.Shutdown(dctx)
	_ = adminSrv.Shutdown(dctx)
	if *keystorePath != "" {
		if err := audit.SealKeys(*keystorePath, unseal, audit.KeyMaterial{SigningSeed: signingSeed, HMACKey: hmacKey, Vault: vault.Snapshot(), WitnessSeed: witnessSeed}); err != nil {
			log.Printf("shutdown: keystore seal FAILED: %v", err)
		} else {
			log.Printf("keystore: node identity + vault sealed on shutdown")
		}
	}
	log.Printf("shutdown: complete")
}

// mustRand fills b with CSPRNG bytes and fails CLOSED on error rather than seeding a
// key/id with predictable/zero material (SoT-31 R4 / audit LOW-2).
func mustRand(b []byte) {
	if _, err := rand.Read(b); err != nil {
		panic("crypto/rand read failed: " + err.Error())
	}
}

// randHex returns n random bytes hex-encoded (dev token generation).
func randHex(n int) string {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		// Fail closed rather than mint predictable dev/admin tokens (SoT-31 R4).
		panic("crypto/rand read failed: " + err.Error())
	}
	const hexd = "0123456789abcdef"
	out := make([]byte, n*2)
	for i, c := range b {
		out[i*2] = hexd[c>>4]
		out[i*2+1] = hexd[c&0xf]
	}
	return string(out)
}

// selfSignedCert generates an in-memory dev certificate (production uses ACME).
func selfSignedCert() (tls.Certificate, error) {
	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return tls.Certificate{}, err
	}
	tmpl := x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject:      pkix.Name{CommonName: "humanymous.local"},
		NotBefore:    time.Now().Add(-time.Hour),
		NotAfter:     time.Now().Add(365 * 24 * time.Hour),
		KeyUsage:     x509.KeyUsageDigitalSignature | x509.KeyUsageKeyEncipherment,
		ExtKeyUsage:  []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		DNSNames:     []string{"localhost", "humanymous.local"},
		IPAddresses:  []net.IP{net.IPv4(127, 0, 0, 1), net.IPv6loopback},
	}
	der, err := x509.CreateCertificate(rand.Reader, &tmpl, &tmpl, &priv.PublicKey, priv)
	if err != nil {
		return tls.Certificate{}, err
	}
	return tls.Certificate{Certificate: [][]byte{der}, PrivateKey: priv}, nil
}
