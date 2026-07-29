package main

import (
	"context"
	"log/slog"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/correlation"
	"github.com/modootoday/humanymous/internal/livefeed"
	"github.com/modootoday/humanymous/internal/mlcorrect"
	"github.com/modootoday/humanymous/internal/network"
	"github.com/modootoday/humanymous/internal/resource"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
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
	passVel   *abuse.Limiter         // SoT-36 §7 axis ③: Pass challenge/solve velocity (short window, no lockout)
	attestLim *abuse.Limiter         // SoT-36 §2 axis ①: attestation-token issuance budget per fingerprint (short window)
	hub       *livefeed.Hub          // SoT-30 live telemetry broadcast (nil unless HMN_PLAYGROUND=1)
	masterKey []byte
	// stepUpKey, when set (from the SHARED HMN_TOKEN_KEY the Gate also reads), signs the
	// ceiling-guard #1 step-up receipt handed to the client on a verified Pass solve. The
	// client redeems it at the Gate's /stepup for the socket-bound hmn_su proof. nil (the
	// standalone-Core default) => no receipt is emitted: there is no Gate to honor it.
	stepUpKey []byte
	webDir    string
	ritOn     bool

	log                   *slog.Logger // PLAN-07 R11 structured logger (DiscardHandler unless -log-level set)
	logEnabled            bool         // fast gate: hot-path verdict/request emits are skipped entirely when off
	externalInputReceipts *externalInputReceiptWriter

	started  time.Time // process start, for /healthz + counters uptime (PLAN-07 R17)
	opsToken string    // operator bearer for /api/explain + /api/counters (empty = routes absent, PLAN-07 R14/R17)

	pass *passStore // SoT-36 humanymous Pass challenge state (per-session, in-memory)

	// ctrl is the SoT-42 Pillar A self-correcting control plane (ACI calibration + drift + Pass
	// poisoning guard). Non-nil ONLY when a -ml-bundle is loaded; every feed/read site nil-guards.
	// It is fed monitor-first from the Pass outcome path (solved ⇒ human oracle) and exposed
	// read-only at /api/mlcorrect. It moves only the weight-0 l4.ml.behavioral annotation, never a
	// verdict.
	ctrl *mlcorrect.Controller

	// SoT-30 Phase-3 local Red launcher (all nil/zero unless HMN_PLAYGROUND=1).
	nonces       *nonceStore
	launchSem    chan struct{} // capacity 1: serializes launches (§10)
	launchMu     sync.Mutex
	launchCancel context.CancelFunc
	runProfile   func(ctx context.Context, profile, base string) ([]byte, error) // injectable for tests

	// lastFP tracks client fingerprintId per session for mid-session fp-churn
	// (rotate-fp-to-dodge-HR-19 residual). Not a cross-session JA4 census.
	lastFPMu sync.Mutex
	lastFP   map[string]string
}

func newApp(webDir string, masterKey []byte, ritOn bool) *app {
	a := &app{
		store:   collector.NewStore(30 * time.Minute),
		engine:  scoring.NewEngine(),
		reg:     newConnRegistry(),
		tlog:    trafficguard.NewLog(30 * time.Minute),
		ledger:  watermark.NewLedger(24 * time.Hour),
		media:   resource.NewMediaTracker(),
		corr:    correlation.New(60 * time.Minute),
		limiter: abuse.NewLimiter(10*time.Second, 40, 80),
		authLim: abuse.NewLimiter(60*time.Second, 5, 10),
		// Short 30s window: velocity taxes cost in the moment, then self-clears fast —
		// NEVER a multi-minute lockout (that would break wargame iteration). soft 4 / hard 8.
		passVel: abuse.NewLimiter(30*time.Second, 4, 8),
		// Attestation issuance budget per fingerprint: at most 3 tokens / 30s. A short,
		// self-clearing window (NOT a ban) — it caps identity throughput, never stalls.
		attestLim: abuse.NewLimiter(30*time.Second, 3, 3),
		masterKey: masterKey,
		webDir:    webDir,
		ritOn:     ritOn,
		pass:      newPassStore(),
		// Default to a disabled logger so an app built without configureLogging (tests,
		// embedders) is silent AND zero-cost. main() flips this via configureLogging.
		log:     slog.New(slog.DiscardHandler),
		started: time.Now(),
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
	mux.HandleFunc("/pass", a.handlePassPage)              // SoT-36 humanymous Pass (demo/research surface)
	mux.HandleFunc("/api/pass/new", a.handlePassNew)       // issue a fresh challenge instance
	mux.HandleFunc("/api/pass/pow", a.handlePassPoW)       // axis ①: non-interactive crypto preflight
	mux.HandleFunc("/api/pass/attest", a.handlePassAttest) // axis ①: rate-limited attestation token issuance
	mux.HandleFunc("/api/pass/solve", a.handlePassSolve)   // verify placement + interaction
	mux.HandleFunc("/api/pass/kpi", a.handlePassKPI)       // wargame KPIs (bypass-rate, human floor)
	mux.HandleFunc("/api/trace", a.handleTrace)
	mux.HandleFunc("/api/traffic/", a.handleTraffic)
	mux.HandleFunc("/res/", a.handleResource)
	mux.Handle("/static/", http.StripPrefix("/static/", http.FileServer(http.Dir(a.webDir))))
	// Ops surface (PLAN-07 R17): /healthz is always-on, unauthenticated liveness.
	mux.HandleFunc("/healthz", a.handleHealthz)
	// PLAN-07 R14/R17: the production explain + counters endpoints are operator-token
	// gated and mounted ONLY when a token is configured (absent = not discoverable).
	if a.opsToken != "" {
		mux.HandleFunc("/api/explain/", a.handleExplain)  // R14: ScoreTrace over a re-scored COPY
		mux.HandleFunc("/api/counters", a.handleCounters) // R17: runtime/ops counters
		mux.HandleFunc("/api/mlcorrect", a.handleMLCorrect) // SoT-42: self-calibration snapshot (no-PII)
	}
	// SoT-30: the read-only Detection Observatory surface, mounted only when the
	// live-telemetry hub is present (HMN_PLAYGROUND=1).
	if a.hub != nil {
		a.registerPlayground(mux)
	}
	return a.withSecurityHeaders(a.withTrafficLog(a.withObservability(mux)))
}

// withTrafficLog records every TCP/TLS/HTTP request into the traffic log so the
// intra-session consistency guard (SoT-12) can detect rotation/evasion.
// RemoteAddr uses clientIP (XFF-aware when the peer is a trusted proxy) so
// WireGuard/OpenVPN exit rotation simulated via trusted XFF is visible as
// l5.traffic.ip_hop — not just the Docker bridge peer that never changes.
func (a *app) withTrafficLog(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		sid := cookieValue(r, sessionCookie)
		hi := reqToHeaderInfo(r)
		client := clientIP(r)
		rec := trafficguard.TrafficRecord{
			TS:         time.Now(),
			SessionID:  sid,
			RemoteAddr: client,
			Method:     r.Method,
			Path:       r.URL.Path,
			Proto:      protoVer(r),
			UAHash:     trafficguard.HashUA(hi.UserAgent),
			// hi.Order() is a SORTED header-SET on this path (OrderReliable=false; net/http
			// drops wire order), so this is a stable set hash, not an order hash. The
			// order-based consumer (l5.traffic.header_order_shift) is DISABLED accordingly.
			HeaderOrder: trafficguard.HashOrder(hi.Order()),
			HasSecFetch: hi.SecFetchPresent(),
		}
		if hello := a.reg.Hello(r.RemoteAddr); hello != nil {
			ja3, _ := network.JA3(hello)
			ja4, _ := network.JA4(hello)
			a.tlog.RecordTLS(trafficguard.TrafficRecord{
				TS: rec.TS, RemoteAddr: client,
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
// datacenterNets is the operator-supplied set of datacenter/cloud IP ranges (from an ASN
// feed or a provider CIDR list). It is EMPTY by default, which makes isDatacenterIP fail
// OPEN — returning false for every IP. This is deliberate (deep-review blocker): the old
// stub returned true for EVERY public IP, so on any real (non-loopback) deployment it fired
// l5.ip.datacenter_asn on every visitor and HR-11 hard-CHALLENGEd 100% of real humans,
// violating the no-lockout constraint. Missing a datacenter bot is far cheaper than
// challenging everyone, so with no dataset wired the signal simply does not fire.
var datacenterNets []*net.IPNet

// SetDatacenterCIDRs wires the real datacenter/cloud IP-range set an operator must supply
// for l5.ip.datacenter_asn to fire in production. Until then isDatacenterIP fails open.
func SetDatacenterCIDRs(nets []*net.IPNet) { datacenterNets = nets }

func isDatacenterIP(ip string) bool {
	p := net.ParseIP(ip)
	if p == nil {
		return false // unresolvable → do not accuse
	}
	if p.IsLoopback() || p.IsPrivate() || p.IsLinkLocalUnicast() || p.IsUnspecified() {
		return false
	}
	for _, n := range datacenterNets {
		if n.Contains(p) {
			return true
		}
	}
	return false // fail open: no dataset ⇒ do not accuse a real user's public IP
}

// proxyVPNNets is the operator-supplied commercial VPN / open-proxy CIDR set.
// Empty by default → fail open. Feeds l5.ip.proxy_vpn_tor (HR-24 under browser UA).
// Tor exits are wired separately via SetTorExitCIDRs → l5.ip.tor_exit.
var proxyVPNNets []*net.IPNet

// SetProxyVPNCIDRs wires commercial VPN / open-proxy ranges for l5.ip.proxy_vpn_tor.
func SetProxyVPNCIDRs(nets []*net.IPNet) { proxyVPNNets = nets }

func isProxyVPNIP(ip string) bool {
	p := net.ParseIP(ip)
	if p == nil {
		return false
	}
	if p.IsLoopback() || p.IsPrivate() || p.IsLinkLocalUnicast() || p.IsUnspecified() {
		return false
	}
	for _, n := range proxyVPNNets {
		if n.Contains(p) {
			return true
		}
	}
	return false
}

// torExitNets is the operator-supplied Tor exit-relay CIDR set (e.g. from the
// public Tor bulk exit list). Empty by default → fail open. Feeds l5.ip.tor_exit.
var torExitNets []*net.IPNet

// SetTorExitCIDRs wires Tor exit-relay ranges for l5.ip.tor_exit (HR-24).
func SetTorExitCIDRs(nets []*net.IPNet) { torExitNets = nets }

func isTorExitIP(ip string) bool {
	p := net.ParseIP(ip)
	if p == nil {
		return false
	}
	if p.IsLoopback() || p.IsPrivate() || p.IsLinkLocalUnicast() || p.IsUnspecified() {
		return false
	}
	for _, n := range torExitNets {
		if n.Contains(p) {
			return true
		}
	}
	return false
}

// fingerprintChurnSignal fires when fingerprintId changes mid-session (same cookie).
// Bots rotate the client-controlled fingerprintId per exit hop to dodge HR-19's
// fingerprint|ja4 correlation key. Cross-session JA4 fan-out is NOT used here —
// ordinary Chrome users share a JA4 prefix and would mass-false-positive.
func fingerprintChurnSignal(sid, fp string, a *app) signals.Signal {
	if sid == "" || fp == "" || a == nil {
		return signals.Signal{}
	}
	a.lastFPMu.Lock()
	defer a.lastFPMu.Unlock()
	if a.lastFP == nil {
		a.lastFP = map[string]string{}
	}
	prev, ok := a.lastFP[sid]
	a.lastFP[sid] = fp
	if ok && prev != "" && prev != fp {
		return signals.New("l5.correlation.fp_churn_proxy",
			map[string]string{"prev": prev, "next": fp},
			signals.VerdictBot, 1.0, signals.SourceServer,
			"fingerprintId rotated mid-session (proxy fp-churn evasion)")
	}
	return signals.Signal{}
}

// vpnWebRTCLeakSignal compares client-reported WebRTC public ICE candidates to the
// server-observed TCP client IP. OpenVPN/WireGuard tunnels often leak the real
// egress via STUN while TCP hits the origin from the tunnel exit.
func vpnWebRTCLeakSignal(adv signals.Advanced, tcpClientIP string) signals.Signal {
	tcp := net.ParseIP(tcpClientIP)
	if tcp == nil || tcp.IsLoopback() || tcp.IsPrivate() || tcp.IsLinkLocalUnicast() {
		// Lab/Docker peers are private; WebRTC leak is only meaningful against a public
		// TCP peer (production). Skip rather than mass-flag containerized tests that
		// did not model a public tunnel exit.
		return signals.Signal{}
	}
	tcpStr := tcp.String()
	for _, raw := range adv.WebRTCPublicAddrs {
		ip := net.ParseIP(strings.TrimSpace(raw))
		if ip == nil || ip.IsLoopback() || ip.IsPrivate() || ip.IsLinkLocalUnicast() || ip.IsUnspecified() {
			continue
		}
		if ip.String() != tcpStr {
			return signals.New("l5.proxy.vpn_webrtc_leak",
				map[string]string{"tcp": tcpStr, "webrtc": ip.String()},
				signals.VerdictBot, 1.0, signals.SourceServer,
				"WebRTC public candidate ≠ TCP client IP (VPN/tunnel leak)")
		}
	}
	return signals.Signal{}
}
