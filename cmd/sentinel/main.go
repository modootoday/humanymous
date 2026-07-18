// Command sentinel is the humanymous Sentinel reverse-proxy security layer (SoT-19..28,31,32):
// (Browser) -> (humanymous Sentinel: TLS terminate + streaming HTML injection +
// L1-L7 scoring + edge enforcement + tamper-evident audit) -> (origin app).
//
// It fronts an existing upstream the operator does not control, injects the
// detection bundle into HTML on the fly, enforces the L7 verdict at the edge,
// and emits every decision into the audit chain before it takes effect.
package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
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
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/sentinel"
)

func main() {
	addr := flag.String("addr", ":8444", "public edge listen address")
	adminAddr := flag.String("admin-addr", ":8445", "SEPARATE admin listener (auth-gated; SoT-28 WS1)")
	upstream := flag.String("upstream", "http://127.0.0.1:9000", "origin upstream base URL")
	node := flag.String("node", "sentinel-1", "node id (audit chain owner)")
	monitor := flag.Bool("monitor", false, "global monitor/shadow mode (score+log, enforce nothing)")
	originKeyHex := flag.String("origin-key", "", "origin-cloaking HMAC key (hex); origin validates X-Hmny-Origin-Auth")
	keystorePath := flag.String("keystore", "", "sealed keystore path for persistent node identity (SoT-28 WS8; needs HMN_UNSEAL); empty = ephemeral")
	// SoT-31 R1 — edge TLS: bring-your-own cert or Let's Encrypt (autocert). None => self-signed.
	tlsCert := flag.String("tls-cert", "", "edge TLS certificate file (PEM); pair with -tls-key for bring-your-own TLS")
	tlsKey := flag.String("tls-key", "", "edge TLS private key file (PEM)")
	acmeDomain := flag.String("acme-domain", "", "comma-separated domain(s) for a Let's Encrypt edge cert via TLS-ALPN-01 (requires binding :443)")
	acmeCache := flag.String("acme-cache", "acme-cache", "directory to cache issued ACME certificates")
	acmeEmail := flag.String("acme-email", "", "optional ACME account contact email")
	// SoT-31 R2 — external route table (prefix -> preset); empty uses the built-in presets.
	routesFile := flag.String("routes", "", "path to a route policy file (`<prefix> <preset>` per line); empty = built-in presets")
	// SoT-32 — durable audit WAL: the tamper-evident chain survives restarts and the
	// in-memory window is bounded. Empty = ephemeral in-memory (dev, unchanged).
	auditWAL := flag.String("audit-wal", "", "durable audit WAL directory (SoT-32); empty = ephemeral in-memory")
	auditVerify := flag.Bool("audit-verify", false, "replay the audit WAL, verify the chain, print the result, and exit")
	auditRedis := flag.String("audit-redis", "", "Redis host:port to project the audit stream to (SoT-32 Tier 1 hot); empty = off")
	auditCH := flag.String("audit-clickhouse", "", "ClickHouse HTTP base URL (e.g. http://ch:8123) to project the audit log to (SoT-32 Tier 2 cold); empty = off")
	flag.Parse()

	originKey := []byte(*originKeyHex)
	if len(originKey) == 0 {
		originKey = make([]byte, 32)
		_, _ = rand.Read(originKey)
	}

	// Audit log FIRST (SoT-18): nothing enforces until decisions have a home.
	// Key persistence (SoT-28 WS8): with -keystore, the STH signing seed + HMAC
	// key + vault linkage keys are sealed and resumed across restarts, so a reboot
	// keeps the same chain identity and the same pseudonym linkage (no accidental
	// mass crypto-shred). Without it, keys are ephemeral (dev).
	var hmacKey, signingSeed []byte
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
		hmacKey, signingSeed, vault = m.HMACKey, m.SigningSeed, audit.LoadVault(m.Vault)
		if created {
			log.Printf("keystore: created new sealed node identity at %s", *keystorePath)
		} else {
			log.Printf("keystore: resumed persisted node identity from %s", *keystorePath)
		}
	} else {
		hmacKey = make([]byte, 32)
		_, _ = rand.Read(hmacKey)
		vault = audit.NewVault()
	}
	vault.SetStretch(true) // KDF-stretch pseudonyms (SoT-28 WS8 brute-force resistance)
	// Independent witness co-signs each Signed Tree Head (SoT-28 WS8) so the
	// writer cannot rewrite history undetected (it can't obtain a witness sig).
	witness := audit.NewWitness()
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
		projections = append(projections, audit.NewCHSink(*auditCH, "audit_log", 10000, time.Second, 100000))
		log.Printf("audit projection: ClickHouse -> %s (audit_log)", *auditCH)
	}
	alog := audit.NewLog(audit.Config{NodeID: *node, HMACKey: hmacKey, CheckpointEvery: 32, Witness: witness, SigningSeed: signingSeed, WAL: auditSink, Projections: projections})
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

	// Persist keys + vault on shutdown so the next boot resumes identity (WS8).
	if *keystorePath != "" {
		go func() {
			ch := make(chan os.Signal, 1)
			signal.Notify(ch, os.Interrupt, syscall.SIGTERM)
			<-ch
			_ = audit.SealKeys(*keystorePath, unseal, audit.KeyMaterial{SigningSeed: signingSeed, HMACKey: hmacKey, Vault: vault.Snapshot()})
			log.Printf("keystore: node identity + vault sealed on shutdown")
			os.Exit(0)
		}()
	}

	// Shared scoring/verdict state (SoT-22 externalizes this to Redis in prod).
	store := collector.NewStore(30 * time.Minute)
	engine := scoring.NewEngine()
	verdicts := sentinel.NewVerdictStore(30 * time.Minute)

	tokenKey := make([]byte, 32)
	_, _ = rand.Read(tokenKey)
	// Shared rotating token epoch (SoT-28 WS6): the control plane mints under the
	// current epoch, the edge accepts current+previous, and a timer rotates.
	epochs := sentinel.NewEpochManager()
	control := sentinel.NewControlPlane(store, engine, verdicts, sink, vault).WithTokenKey(tokenKey).WithTokenEpochs(epochs)

	cfg := sentinel.Config{
		Upstream:      *upstream,
		NodeID:        *node,
		ControlPath:   "/__hmn/",
		GlobalMonitor: *monitor,
		OriginKey:     originKey,
		TokenKey:      tokenKey,
		TokenEpochs:   epochs,
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

	srv, err := sentinel.NewServer(cfg, sink, vault, verdicts, control.Handler())
	if err != nil {
		log.Fatalf("proxy: %v", err)
	}

	// Seed admin operators with bearer tokens (SoT-28 WS1/WS2). Production issues
	// these via mTLS client certs or an SSO/operator-CA login; the reference
	// prints dev tokens at startup. The console is injected with the operator
	// token so it can read + request; approvals need a distinct approver token.
	toks := map[sentinel.Role]string{}
	// Deterministic dev tokens via HMN_ADMIN_TOKENS="auditor:tok,operator:tok,..."
	// (for e2e/demo); otherwise generate random per boot.
	envToks := map[string]string{}
	for _, kv := range strings.Split(os.Getenv("HMN_ADMIN_TOKENS"), ",") {
		if p := strings.SplitN(kv, ":", 2); len(p) == 2 {
			envToks[p[0]] = p[1]
		}
	}
	// SoT-31 R3 — fail-safe: refuse to boot with the shipped `e2e-*` demo tokens
	// unless the demo/war explicitly opts in (HMN_ALLOW_DEV_TOKENS=1). An adopter
	// who copies configs/dev.env is stopped, not silently exposed on :8445.
	if os.Getenv("HMN_ALLOW_DEV_TOKENS") != "1" {
		for role, v := range envToks {
			if strings.HasPrefix(v, "e2e-") {
				log.Fatalf("refusing to boot: HMN_ADMIN_TOKENS carries a shipped dev token for %q; set real admin tokens, or HMN_ALLOW_DEV_TOKENS=1 for the local demo only (SoT-31 R3)", role)
			}
		}
	}
	for _, role := range []sentinel.Role{sentinel.RoleAuditor, sentinel.RoleOperator, sentinel.RoleApprover, sentinel.RoleDPO} {
		t := envToks[string(role)]
		if t == "" {
			t = randHex(24)
		}
		toks[role] = t
		srv.Auth().Add(t, string(role)+"-1", role)
	}
	srv.SetDevConsoleToken(toks[sentinel.RoleOperator])

	// SoT-31 R1 — the public edge serves a real cert (BYO / ACME) or self-signed;
	// the admin plane stays self-signed (management-network + mTLS is the real
	// control), so ACME challenges never depend on the admin port.
	edgeCfg, err := buildEdgeTLS(edgeTLS{
		acmeDomains: splitDomains(*acmeDomain), acmeCache: *acmeCache, acmeEmail: *acmeEmail,
		certFile: *tlsCert, keyFile: *tlsKey,
	})
	if err != nil {
		log.Fatalf("edge tls: %v", err)
	}
	adminCert, err := selfSignedCert()
	if err != nil {
		log.Fatalf("admin cert: %v", err)
	}
	adminCfg := &tls.Config{Certificates: []tls.Certificate{adminCert}, MinVersion: tls.VersionTLS12}
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
	// Rotate the verdict-token epoch periodically (SoT-28 WS6): bounds how long a
	// cloned token survives (accepted for at most one rotation after issuance).
	go func() {
		t := time.NewTicker(15 * time.Minute)
		for range t.C {
			epochs.Advance()
		}
	}()

	adminSrv := mkServer(*adminAddr, srv.AdminHandler(), adminCfg)
	go func() {
		log.Printf("humanymous Sentinel admin console on https://localhost%s/__hmn/admin/console", *adminAddr)
		log.Printf("  dev tokens — auditor:%s operator:%s approver:%s dpo:%s", toks[sentinel.RoleAuditor], toks[sentinel.RoleOperator], toks[sentinel.RoleApprover], toks[sentinel.RoleDPO])
		log.Fatalf("admin listener: %v", adminSrv.ListenAndServeTLS("", ""))
	}()

	pubSrv := mkServer(*addr, srv, edgeCfg)
	tlsMode := "self-signed"
	if len(splitDomains(*acmeDomain)) > 0 {
		tlsMode = "ACME " + *acmeDomain
	} else if *tlsCert != "" && *tlsKey != "" {
		tlsMode = "BYO cert"
	}
	log.Printf("humanymous Sentinel on https://localhost%s -> %s (monitor=%v, tls=%s)", *addr, *upstream, *monitor, tlsMode)
	log.Fatal(pubSrv.ListenAndServeTLS("", ""))
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
