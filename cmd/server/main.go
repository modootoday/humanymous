// SPDX-License-Identifier: Apache-2.0

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
	"strconv"
	"time"

	"github.com/modootoday/humanymous/internal/mlcorrect"
	"github.com/modootoday/humanymous/internal/mlserve"
	"golang.org/x/net/http2"
	"golang.org/x/net/netutil"
)

// envInt returns the integer value of env var name, or def if unset/invalid.
func envInt(name string, def int) int {
	if v := os.Getenv(name); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return def
}

// version is stamped at release via -ldflags "-X main.version=<tag>-<sha>-<date>"
// so a running engine self-reports which build it is (audit LOW-4).
var version = "dev"

func main() {
	addr := flag.String("addr", ":8443", "listen address")
	maxConns := flag.Int("max-conns", envInt("HMN_MAX_CONNS", 1024), "max concurrent accepted connections (slowloris/slow-POST accumulation cap; 0 disables)")
	webDir := flag.String("web", "web", "web asset directory")
	ritOn := flag.Bool("rit", true, "enable RIT anti-tamper token verification")
	acmeDomain := flag.String("acme-domain", "", "comma-separated domain(s) for a Let's Encrypt cert via TLS-ALPN-01 (requires binding :443 directly; empty = self-signed)")
	acmeCache := flag.String("acme-cache", "acme-cache", "directory to cache issued ACME certificates")
	acmeEmail := flag.String("acme-email", "", "optional contact email for the ACME account")
	tlsCert := flag.String("tls-cert", "", "operator-provided TLS certificate chain; requires -tls-key and cannot be combined with ACME")
	tlsKey := flag.String("tls-key", "", "operator-provided TLS private key; requires -tls-cert and cannot be combined with ACME")
	logLevel := flag.String("log-level", "", "operational log level: off|debug|info|warn|error (default off; also HMN_LOG_LEVEL)")
	logConsoleFormat := flag.String("log-console-format", "", "console log format: off|plain|jsonl (default plain; also HMN_LOG_CONSOLE_FORMAT)")
	logConsoleStream := flag.String("log-console-stream", "", "console log stream: stderr|stdout (default stderr; also HMN_LOG_CONSOLE_STREAM)")
	logPlainFile := flag.String("log-plain-file", "", "append formatted plain-text logs to PATH (also HMN_LOG_PLAIN_FILE)")
	logJSONLFile := flag.String("log-jsonl-file", "", "append JSON Lines logs to PATH (also HMN_LOG_JSONL_FILE)")
	externalInputReceiptDir := flag.String("external-input-receipt-dir", "", "lab-only directory for run-bound Core score receipts")
	opsToken := flag.String("ops-token", "", "operator bearer token enabling /api/explain + /api/counters (empty = disabled; also HMN_OPS_TOKEN)")
	mlBundle := flag.String("ml-bundle", os.Getenv("HMN_ML_BUNDLE"), "path to a behavioral model bundle (SoT-42 Pillar A); empty = no model (l4.ml.behavioral abstains, engine unchanged)")
	mlFPBudget := flag.Float64("ml-fp-budget", 0.005, "SoT-42 self-calibration: tolerated human false-positive rate the ML residual's threshold maintains (only used when -ml-bundle is set)")
	flag.Parse()
	explicitFlags := make(map[string]bool)
	flag.Visit(func(current *flag.Flag) {
		explicitFlags[current.Name] = true
	})

	// SoT-42 Pillar A — load the behavioral model bundle if configured. A load failure is
	// NON-FATAL: the seam stays in Abstain, so a missing/corrupt/foreign bundle degrades to the
	// heuristics-only engine rather than taking the edge down (fail-open on the model, never block
	// a human on model plumbing). With no bundle the l4.ml.behavioral residual never fires.
	var mlCtrl *mlcorrect.Controller
	if *mlBundle != "" {
		if m, err := mlserve.LoadMLP(*mlBundle); err != nil {
			log.Printf("ml-bundle: load failed (%v) — behavioral model disabled, engine runs on heuristics", err)
		} else {
			mlserve.Set(m)
			// SoT-42 Pillar A — install the self-calibrating control plane as the residual's fire
			// threshold (monitor-first). It starts at θ0=0.5 (identical to the prior static cut) and
			// only self-adjusts from oracle-confirmed outcomes fed on the verdict path; because
			// l4.ml.behavioral is weight-0/score-exempt, this moves only the audit annotation, never a
			// verdict. With no bundle the provider is never installed and FireThreshold() stays 0.5.
			mlCtrl = mlcorrect.NewController(float32(*mlFPBudget), 0.5, 0.01)
			mlserve.SetThresholdProvider(mlCtrl)
			log.Printf("ml-bundle: loaded %s (schema %s); self-calibration active (budget=%.4f θ0=%.2f)",
				m.BundleVersion(), m.SchemaHash(), *mlFPBudget, mlCtrl.FireThreshold())
		}
	}

	domains := splitDomains(*acmeDomain)

	// SoT-30 §11.1 — fail-closed: the Detection Observatory is loopback-only. If
	// its dev flag is on, refuse to bind a non-loopback address, so the surface
	// can never become remotely reachable (rewriting the launcher host is not
	// enough if the socket itself is reachable off-box).
	if playgroundEnabled() && !listenAddrIsLoopback(*addr) {
		log.Fatalf("HMN_PLAYGROUND=1 requires a loopback -addr; refusing to expose the self-validation launcher off-host")
	}
	// The dev playground and public ACME serving are mutually exclusive: one is
	// loopback-only, the other is a public domain on :443. Refuse the mix.
	if playgroundEnabled() && len(domains) > 0 {
		log.Fatalf("HMN_PLAYGROUND=1 is loopback-only and cannot be combined with -acme-domain (public TLS)")
	}

	masterKey := make([]byte, 32)
	if _, err := rand.Read(masterKey); err != nil {
		log.Fatalf("master key generation failed")
	}

	a := newApp(*webDir, masterKey, *ritOn)
	a.ctrl = mlCtrl // SoT-42 self-correcting control plane (nil unless a bundle loaded)
	if err := a.configureExternalInputReceipts(*externalInputReceiptDir); err != nil {
		log.Fatal("external-input receipt configuration failed")
	}
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
	logRuntime, err := a.configureLogging(coreLoggingConfig{
		Level:         resolveLogSetting(*logLevel, explicitFlags["log-level"], "HMN_LOG_LEVEL", "off"),
		ConsoleFormat: resolveLogSetting(*logConsoleFormat, explicitFlags["log-console-format"], "HMN_LOG_CONSOLE_FORMAT", "plain"),
		ConsoleStream: resolveLogSetting(*logConsoleStream, explicitFlags["log-console-stream"], "HMN_LOG_CONSOLE_STREAM", "stderr"),
		PlainFile:     resolveLogSetting(*logPlainFile, explicitFlags["log-plain-file"], "HMN_LOG_PLAIN_FILE", ""),
		JSONLFile:     resolveLogSetting(*logJSONLFile, explicitFlags["log-jsonl-file"], "HMN_LOG_JSONL_FILE", ""),
	})
	if err != nil {
		log.Fatal("operational logging configuration failed")
	}
	defer closeOperationalLogger(logRuntime)
	a.configureOps(*opsToken) // PLAN-07 R14/R17: operator-gated explain + counters (off by default)

	tlsConfig, err := buildTLSConfig(tlsSettings{
		acmeDomains: domains,
		acmeCache:   *acmeCache,
		acmeEmail:   *acmeEmail,
		certFile:    *tlsCert,
		keyFile:     *tlsKey,
	})
	if err != nil {
		a.log.Error("TLS configuration failed.",
			"component", "core.runtime",
			"event", "runtime.tls_configuration_failed")
		closeOperationalLogger(logRuntime)
		os.Exit(1)
	}
	handler := a.routes()

	base, err := net.Listen("tcp", *addr)
	if err != nil {
		a.log.Error("Listener startup failed.",
			"component", "core.runtime",
			"event", "runtime.listener_failed")
		closeOperationalLogger(logRuntime)
		os.Exit(1)
	}
	// Bound the number of concurrently ACCEPTED connections. The per-connection
	// ReadHeaderTimeout (below) caps each slow connection's DURATION, but without
	// this a slowloris / slow-POST flood can still ACCUMULATE unbounded concurrent
	// connections (open-rate × hold-time), each tying up a goroutine + TLS buffers
	// (SoT-17). LimitListener holds excess connections in the OS backlog until a
	// slot frees, converting unbounded accumulation into bounded back-pressure. The
	// default is generous so it only trips under attack, not normal burst load.
	var lst net.Listener = base
	if *maxConns > 0 {
		lst = netutil.LimitListener(base, *maxConns)
	}
	capL := &captureListener{Listener: lst, reg: a.reg}

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

	tlsMode := "self_signed"
	if len(domains) > 0 {
		tlsMode = "acme"
	} else if *tlsCert != "" {
		tlsMode = "operator_provided"
	}
	a.log.Info("Core listener started.",
		"component", "core.runtime",
		"event", "runtime.started",
		"tls_mode", tlsMode,
		"rit_enabled", *ritOn)

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
			a.log.Error("Listener accept failed.",
				"component", "core.runtime",
				"event", "runtime.accept_failed")
			closeOperationalLogger(logRuntime)
			os.Exit(1)
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
		a.log.Debug("Connection protocol negotiated.",
			"component", "core.http",
			"event", "http.protocol_negotiated",
			"protocol", proto)
	}
	if proto == "h2" {
		fp, r, perr := peekH2(tconn)
		if fp != nil {
			a.reg.SetH2(addr, fp)
			a.reg.SetHeaderOrder(addr, fp.HeaderOrder)
		}
		if debugH2 {
			settingsCount := 0
			pseudoCount := 0
			if fp != nil {
				settingsCount = len(fp.Settings)
				pseudoCount = len(fp.PseudoOrder)
			}
			a.log.Debug("HTTP/2 preface inspected.",
				"component", "core.http",
				"event", "http.h2_preface_inspected",
				"success", perr == nil,
				"settings_count", settingsCount,
				"pseudo_header_count", pseudoCount)
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
	// HTTP/1.1: serve this single connection via a one-shot listener. These
	// timeouts bound each slow connection's DURATION; the LimitListener at the
	// accept loop bounds concurrent ACCUMULATION (both SoT-17 slowloris /
	// slow-POST defenses — duration + concurrency).
	srv := &http.Server{
		Handler:           handler,
		ReadHeaderTimeout: 8 * time.Second,  // slowloris (slow headers)
		ReadTimeout:       30 * time.Second, // slow-POST (slow body)
		WriteTimeout:      30 * time.Second,
		IdleTimeout:       60 * time.Second,
		MaxHeaderBytes:    1 << 20,
	}
	// Capture the on-wire header ORDER (SoT-02 / R4) before net/http maps it, then replay
	// the consumed bytes so the standard server serves the request unchanged.
	order, r, _ := peekH1HeaderOrder(tconn)
	a.reg.SetHeaderOrder(addr, order)
	_ = srv.Serve(&oneConnListener{conn: replayReadConn{Conn: tconn, r: r}})
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
