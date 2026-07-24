// Command server is the humanymous Core detection engine: it captures TLS/HTTP
// signals, merges them with the WASM/JS client report, scores the session and
// returns a verdict. See sots/ and plan/ for the full design.
package main

import (
	"crypto/rand"
	"crypto/tls"
	"encoding/hex"
	"flag"
	"log"
	"net"
	"net/http"
	"os"
	"time"

	"golang.org/x/net/http2"
)

// version is stamped at release via -ldflags "-X main.version=<tag>-<sha>-<date>"
// so a running engine self-reports which build it is (audit LOW-4).
var version = "dev"

func main() {
	addr := flag.String("addr", ":8443", "listen address")
	webDir := flag.String("web", "web", "web asset directory")
	ritOn := flag.Bool("rit", true, "enable RIT anti-tamper token verification")
	acmeDomain := flag.String("acme-domain", "", "comma-separated domain(s) for a Let's Encrypt cert via TLS-ALPN-01 (requires binding :443 directly; empty = self-signed)")
	acmeCache := flag.String("acme-cache", "acme-cache", "directory to cache issued ACME certificates")
	acmeEmail := flag.String("acme-email", "", "optional contact email for the ACME account")
	logLevel := flag.String("log-level", "", "structured log level: off|error|warn|info|debug (default off; also HMN_LOG_LEVEL)")
	opsToken := flag.String("ops-token", "", "operator bearer token enabling /api/explain + /api/counters (empty = disabled; also HMN_OPS_TOKEN)")
	flag.Parse()

	domains := splitDomains(*acmeDomain)

	// SoT-30 §11.1 — fail-closed: the Detection Observatory is loopback-only. If
	// its dev flag is on, refuse to bind a non-loopback address, so the surface
	// can never become remotely reachable (rewriting the launcher host is not
	// enough if the socket itself is reachable off-box).
	if playgroundEnabled() && !listenAddrIsLoopback(*addr) {
		log.Fatalf("HMN_PLAYGROUND=1 requires a loopback -addr (got %q); refusing non-loopback bind (SoT-30 §11.1)", *addr)
	}
	// The dev playground and public ACME serving are mutually exclusive: one is
	// loopback-only, the other is a public domain on :443. Refuse the mix.
	if playgroundEnabled() && len(domains) > 0 {
		log.Fatalf("HMN_PLAYGROUND=1 is loopback-only and cannot be combined with -acme-domain (public TLS)")
	}

	masterKey := make([]byte, 32)
	if _, err := rand.Read(masterKey); err != nil {
		log.Fatalf("master key: %v", err)
	}

	a := newApp(*webDir, masterKey, *ritOn)
	// Ceiling-guard #1: when the Core serves the SoT-36 Pass as an attestation front-end
	// behind a Gate, a solved Pass must produce a step-up receipt the Gate can verify. Read
	// the SHARED HMN_TOKEN_KEY (the same var the Gate uses); absent it, no receipt is
	// emitted (a standalone Core has no Gate to redeem one).
	if tk := os.Getenv("HMN_TOKEN_KEY"); tk != "" {
		b, err := hex.DecodeString(tk)
		if err != nil || len(b) < 16 {
			// Fail closed, symmetric with the Gate (cmd/gate/main.go): a malformed shared key
			// would silently disable step-up receipts and leave attested routes an unredeemable
			// Pass loop, so refuse to start rather than degrade invisibly.
			log.Fatalf("HMN_TOKEN_KEY must be hex of at least 16 bytes")
		}
		a.stepUpKey = b
	}
	a.configureLogging(*logLevel) // PLAN-07 R11: opt-in structured logging (off by default)
	a.configureOps(*opsToken)     // PLAN-07 R14/R17: operator-gated explain + counters (off by default)

	tlsConfig, err := buildTLSConfig(tlsSettings{
		acmeDomains: domains,
		acmeCache:   *acmeCache,
		acmeEmail:   *acmeEmail,
	})
	if err != nil {
		log.Fatalf("tls: %v", err)
	}
	handler := a.routes()

	base, err := net.Listen("tcp", *addr)
	if err != nil {
		log.Fatalf("listen: %v", err)
	}
	capL := &captureListener{Listener: base, reg: a.reg}

	// Background session GC.
	go func() {
		t := time.NewTicker(time.Minute)
		for range t.C {
			now := time.Now()
			a.store.GC(now)
			a.tlog.GC(now)
			a.ledger.GC(now)
			a.corr.GC(now)    // PLAN-07 R2: bound the correlation map (was never swept)
			a.limiter.GC(now) // PLAN-07 R2: bound the fingerprint rate-limiter map
		}
	}()

	if len(domains) > 0 {
		log.Printf("humanymous Core on %s (rit=%v) — ACME TLS for %v (cache %q)", *addr, *ritOn, domains, *acmeCache)
	} else {
		log.Printf("humanymous Core %s on https://localhost%s (rit=%v) — self-signed TLS", version, *addr, *ritOn)
	}

	// Custom accept loop: we terminate TLS ourselves so we can peek the HTTP/2
	// preface + SETTINGS + pseudo-header order (the Akamai fingerprint) before
	// handing the connection to the h2 server (SoT-02 §HTTP/2, plan/02 §3.2).
	// HTTP/2 hardening (SoT-17): cap concurrent streams and frame sizes. Go's h2
	// server already rate-limits RST_STREAM (CVE-2023-44487 Rapid Reset) and
	// bounds CONTINUATION frames; these caps add defense-in-depth.
	h2srv := &http2.Server{
		MaxConcurrentStreams:         100,
		MaxReadFrameSize:             1 << 20,
		MaxUploadBufferPerConnection: 1 << 20,
	}
	for {
		raw, err := capL.Accept()
		if err != nil {
			log.Fatalf("accept: %v", err)
		}
		go a.serveConn(raw, tlsConfig, handler, h2srv)
	}
}

// serveConn terminates TLS, captures the HTTP/2 fingerprint when negotiated, and
// serves the connection with the appropriate protocol handler.
func (a *app) serveConn(raw net.Conn, cfg *tls.Config, handler http.Handler, h2srv *http2.Server) {
	tconn := tls.Server(raw, cfg)
	if err := tconn.Handshake(); err != nil {
		tconn.Close()
		return
	}
	addr := raw.RemoteAddr().String()
	proto := tconn.ConnectionState().NegotiatedProtocol
	if debugH2 {
		log.Printf("serveConn %s proto=%q", addr, proto)
	}
	if proto == "h2" {
		fp, r, perr := peekH2(tconn)
		if fp != nil {
			a.reg.SetH2(addr, fp)
		}
		if debugH2 {
			log.Printf("peekH2 %s err=%v settings=%d pseudo=%v", addr, perr, len(fp.Settings), fp.PseudoOrder)
		}
		rc := replayConn{
			Conn: tconn,
			r:    r,
			mon:  newFrameMonitor(),
			onAbuse: func(sig string) {
				a.reg.SetAbuse(addr, sig)
				// SoT-30 tap B: an L5-only / connection-level abuse (Rapid Reset,
				// CONTINUATION flood) never completes a scored /api/collect, so tap A
				// stays silent. Publish it here so the observatory pipeline is not
				// blank for exactly the network-layer attacks (§4.3).
				if a.hub != nil {
					a.hub.Publish("network.abuse", map[string]any{"signal": sig})
				}
			},
		}
		h2srv.ServeConn(rc, &http2.ServeConnOpts{Handler: handler})
		return
	}
	// HTTP/1.1: serve this single connection via a one-shot listener. Timeouts
	// bound slowloris / slow-POST connection-exhaustion attacks (SoT-17).
	srv := &http.Server{
		Handler:           handler,
		ReadHeaderTimeout: 8 * time.Second,  // slowloris (slow headers)
		ReadTimeout:       30 * time.Second, // slow-POST (slow body)
		WriteTimeout:      30 * time.Second,
		IdleTimeout:       60 * time.Second,
		MaxHeaderBytes:    1 << 20,
	}
	_ = srv.Serve(&oneConnListener{conn: tconn})
}

// oneConnListener yields a single connection then signals the server to stop
// accepting (returning ErrServerClosed so Serve exits cleanly).
type oneConnListener struct {
	conn net.Conn
	done bool
}

func (l *oneConnListener) Accept() (net.Conn, error) {
	if l.done {
		return nil, http.ErrServerClosed
	}
	l.done = true
	return l.conn, nil
}
func (l *oneConnListener) Close() error   { return nil }
func (l *oneConnListener) Addr() net.Addr { return l.conn.LocalAddr() }

// debugH2 toggles verbose h2-capture logging (set via HM_DEBUG_H2=1).
var debugH2 = os.Getenv("HM_DEBUG_H2") == "1"
