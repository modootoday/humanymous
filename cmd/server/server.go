package main

import (
	"context"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/correlation"
	"github.com/modootoday/humanymous/internal/livefeed"
	"github.com/modootoday/humanymous/internal/network"
	"github.com/modootoday/humanymous/internal/resource"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/trafficguard"
	"github.com/modootoday/humanymous/internal/watermark"
)

// server.go wires the app dependencies and routes (SRP: routing here, request
// logic in handlers.go).

type app struct {
	store     *collector.Store
	engine    *scoring.Engine
	reg       *connRegistry
	tlog      *trafficguard.Log      // SoT-12 traffic logging & consistency
	ledger    *watermark.Ledger      // SoT-08 watermark issuance ledger
	media     *resource.MediaTracker // SoT-10 media abuse tracking
	corr      *correlation.Registry  // SoT-15 cross-session correlation
	limiter   *abuse.Limiter         // SoT-17 fingerprint-keyed rate/flood limiter
	authLim   *abuse.Limiter         // SoT-17 failed-auth velocity (credential stuffing)
	hub       *livefeed.Hub          // SoT-30 live telemetry broadcast (nil unless HMN_PLAYGROUND=1)
	masterKey []byte
	webDir    string
	ritOn     bool

	pass *passStore // SoT-36 humanymous Pass challenge state (per-session, in-memory)

	// SoT-30 Phase-3 local Red launcher (all nil/zero unless HMN_PLAYGROUND=1).
	nonces       *nonceStore
	launchSem    chan struct{} // capacity 1: serializes launches (§10)
	launchMu     sync.Mutex
	launchCancel context.CancelFunc
	runProfile   func(ctx context.Context, profile, base string) ([]byte, error) // injectable for tests
}

func newApp(webDir string, masterKey []byte, ritOn bool) *app {
	a := &app{
		store:     collector.NewStore(30 * time.Minute),
		engine:    scoring.NewEngine(),
		reg:       newConnRegistry(),
		tlog:      trafficguard.NewLog(30 * time.Minute),
		ledger:    watermark.NewLedger(24 * time.Hour),
		media:     resource.NewMediaTracker(),
		corr:      correlation.New(60 * time.Minute),
		limiter:   abuse.NewLimiter(10*time.Second, 40, 80),
		authLim:   abuse.NewLimiter(60*time.Second, 5, 10),
		masterKey: masterKey,
		webDir:    webDir,
		ritOn:     ritOn,
		pass:      newPassStore(),
	}
	// SoT-30 Detection Observatory: the live-telemetry hub exists only when the
	// dev flag is set, so gate-off leaves the scorer tap a zero-cost nil check.
	if playgroundEnabled() {
		a.hub = livefeed.New(256, 256)
		a.nonces = newNonceStore()
		a.launchSem = make(chan struct{}, 1)
		a.runProfile = realRunProfile
	}
	return a
}

func (a *app) routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/", a.handlePage)
	mux.HandleFunc("/demo", a.handleDemo)
	mux.HandleFunc("/api/session", a.handleSession)
	mux.HandleFunc("/api/collect", a.handleCollect)
	mux.HandleFunc("/api/report/", a.handleReportOne)
	mux.HandleFunc("/api/report", a.handleReportList)
	mux.HandleFunc("/api/csp-report", a.handleCSPReport)
	mux.HandleFunc("/api/pow", a.handlePoW)
	mux.HandleFunc("/pass", a.handlePassPage)          // SoT-36 humanymous Pass (demo/research surface)
	mux.HandleFunc("/api/pass/new", a.handlePassNew)   // issue a fresh challenge instance
	mux.HandleFunc("/api/pass/solve", a.handlePassSolve) // verify placement + interaction
	mux.HandleFunc("/api/pass/kpi", a.handlePassKPI)     // wargame KPIs (bypass-rate, human floor)
	mux.HandleFunc("/api/trace", a.handleTrace)
	mux.HandleFunc("/api/traffic/", a.handleTraffic)
	mux.HandleFunc("/res/", a.handleResource)
	mux.Handle("/static/", http.StripPrefix("/static/", http.FileServer(http.Dir(a.webDir))))
	// SoT-30: the read-only Detection Observatory surface, mounted only when the
	// live-telemetry hub is present (HMN_PLAYGROUND=1).
	if a.hub != nil {
		a.registerPlayground(mux)
	}
	return a.withSecurityHeaders(a.withTrafficLog(mux))
}

// withTrafficLog records every TCP/TLS/HTTP request into the traffic log so the
// intra-session consistency guard (SoT-12) can detect rotation/evasion.
func (a *app) withTrafficLog(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		sid := cookieValue(r, sessionCookie)
		hi := reqToHeaderInfo(r)
		rec := trafficguard.TrafficRecord{
			TS:          time.Now(),
			SessionID:   sid,
			RemoteAddr:  r.RemoteAddr,
			Method:      r.Method,
			Path:        r.URL.Path,
			Proto:       protoVer(r),
			UAHash:      trafficguard.HashUA(hi.UserAgent),
			HeaderOrder: trafficguard.HashOrder(hi.Order()),
			HasSecFetch: hi.SecFetchPresent(),
		}
		if hello := a.reg.Hello(r.RemoteAddr); hello != nil {
			ja3, _ := network.JA3(hello)
			ja4, _ := network.JA4(hello)
			a.tlog.RecordTLS(trafficguard.TrafficRecord{
				TS: rec.TS, RemoteAddr: r.RemoteAddr,
				JA3: ja3, JA4: ja4, JA4Engine: network.EngineFromClientHello(hello),
				ExtOrder: network.ExtOrderHash(hello),
			})
		}
		if sid != "" {
			a.tlog.RecordHTTP(rec)
		}
		next.ServeHTTP(w, r)
	})
}

func protoVer(r *http.Request) string {
	if r.ProtoMajor == 2 {
		return "2"
	}
	return "1.1"
}

// withSecurityHeaders applies a strict CSP (SoT-11) to every response. The demo
// uses Report-Only so it never breaks the page while still collecting reports.
func (a *app) withSecurityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		csp := strings.Join([]string{
			"default-src 'self'",
			"script-src 'self' 'wasm-unsafe-eval'",
			"object-src 'none'",
			"base-uri 'none'",
			"report-uri /api/csp-report",
		}, "; ")
		w.Header().Set("Content-Security-Policy-Report-Only", csp)
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
		next.ServeHTTP(w, r)
	})
}

// isDatacenterIP is a stub IP-intel classifier (SoT-02 §6): loopback, private,
// and link-local addresses are never datacenter (LAN, reverse-proxy hops, the
// local Docker demo); any other (public) address is treated as datacenter until
// a real ASN dataset is wired in. Uses net.IP ranges so ALL private blocks are
// excluded — including 172.16/12, which Docker bridge networks use and the old
// string-prefix check missed (it flagged every containerized session as a
// datacenter IP, a persistent +score even for a real user's browser).
func isDatacenterIP(ip string) bool {
	p := net.ParseIP(ip)
	if p == nil {
		return false // unresolvable → do not accuse
	}
	if p.IsLoopback() || p.IsPrivate() || p.IsLinkLocalUnicast() || p.IsUnspecified() {
		return false
	}
	return true
}
