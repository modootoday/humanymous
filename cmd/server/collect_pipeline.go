package main

import (
	"encoding/json"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/network"
	"github.com/modootoday/humanymous/internal/signals"
	"github.com/modootoday/humanymous/internal/trafficguard"
)

// collect_pipeline.go decomposes the /api/collect flow into one method per DOMAIN
// (SoT-00 §5): RIT anti-tamper, network merge, server-side signal enrichment,
// scoring, and the live-feed publish. handleCollect (handlers.go) orchestrates
// these in order; each step here owns a single concern and is testable in
// isolation, instead of one 100-line handler mixing seven domains inline.

// verifyAndRotateRIT verifies the RIT anti-tamper token over the request body and
// rotates the seed for the client's next request (SoT-07). Returns the RIT signal
// to append after MergeNetwork; a zero Signal (empty ID) when RIT is off.
func (a *app) verifyAndRotateRIT(w http.ResponseWriter, r *http.Request, sid string, body []byte) signals.Signal {
	if !a.ritOn {
		return signals.Signal{}
	}
	sig := a.verifyRIT(sid, r, body)
	seed, n, wnd := a.issueSeed(sid)
	w.Header().Set("X-HM-Seed", seed)
	w.Header().Set("X-HM-N", strconv.FormatUint(n, 10))
	w.Header().Set("X-HM-W", strconv.Itoa(wnd))
	return sig
}

// parseClientReport unmarshals the client report and binds the session id. It
// writes a 400 and returns ok=false on a malformed body.
func parseClientReport(w http.ResponseWriter, body []byte, sid string) (signals.ClientReport, bool) {
	var client signals.ClientReport
	if err := json.Unmarshal(body, &client); err != nil {
		http.Error(w, "bad body", http.StatusBadRequest)
		return client, false
	}
	client.SessionID = sid
	return client, true
}

// mergeObservation builds the L5 network observation from the captured
// ClientHello + HTTP/2 fingerprint + headers and merges it with the client
// report into the session (network is pinned on the first collect).
func (a *app) mergeObservation(sid string, r *http.Request, client signals.ClientReport, now time.Time) {
	cip := clientIP(r)
	hi := reqToHeaderInfo(r)
	// Prefer the on-wire header ORDER captured before net/http mapped it (h1 raw peek /
	// h2 HEADERS frame). When present, Names carries wire order and OrderReliable is true,
	// so the header-order-vs-browser check (SoT-02 / R4) becomes observable.
	if order := a.reg.HeaderOrder(r.RemoteAddr); len(order) > 0 {
		hi.Names = order
		hi.OrderReliable = true
	}
	obs := network.Observation{
		Hello:             a.reg.Hello(r.RemoteAddr),
		H2:                a.reg.H2(r.RemoteAddr),
		Header:            hi,
		IsDatacenterIP:    isDatacenterIP(cip),
		IsProxy:           isProxyVPNIP(cip),
		IsTorExit:         isTorExitIP(cip),
		ClientForwardedIP: forwardedIP(r),
	}
	a.store.MergeNetwork(sid, network.Build(obs), now)
	a.store.MergeClient(sid, client, now)
}

// enrichServerSignals appends every server-derived L5 signal to the pinned
// network report, AFTER MergeNetwork (which replaces Network.Signals on the first
// collect). Each block is one detection domain: RIT (SoT-07), intra-session
// traffic consistency (SoT-12), HTTP/2 DoS (SoT-17), fingerprint-keyed rate limit
// (SoT-17), cross-session correlation (SoT-15), and a prior PoW upgrade (SoT-13).
func (a *app) enrichServerSignals(sid string, r *http.Request, client signals.ClientReport, ritSig signals.Signal, now time.Time) {
	if ritSig.ID != "" {
		a.store.AppendNetworkSignals(sid, []signals.Signal{ritSig}, now)
	}
	if traffic := trafficguard.Check(a.tlog.Session(sid)); len(traffic) > 0 {
		a.store.AppendNetworkSignals(sid, traffic, now)
	}
	if dos := a.reg.Abuse(r.RemoteAddr); dos != "" {
		a.store.AppendNetworkSignals(sid, []signals.Signal{
			signals.New(dos, true, signals.VerdictBot, 1.0, signals.SourceServer, "HTTP/2 frame-abuse DoS"),
		}, now)
	}
	// Coalesce the fingerprint derivation (PLAN-07 R19): the stable JA4 and the client
	// subnet are each needed by both the rate limiter and the correlation check. Hello()
	// is a registry lookup and clientSubnet() re-parses the peer address — compute each
	// once and reuse, instead of recomputing per consumer.
	ja4 := ja4Stable(a.reg.Hello(r.RemoteAddr))
	subnet := clientSubnet(r)
	if lvl := a.limiter.Level(a.limiter.Observe(ja4+"|"+subnet, now)); lvl > 0 {
		id, v := "l5.abuse.rate_exceeded", signals.VerdictSuspicious
		if lvl == 2 {
			id, v = "l5.abuse.flood", signals.VerdictBot
		}
		a.store.AppendNetworkSignals(sid, []signals.Signal{
			signals.New(id, nil, v, 1.0, signals.SourceServer, "fingerprint-keyed request rate exceeded"),
		}, now)
	}
	// Cross-session correlation key. The client FingerprintID is attacker-controllable, so a
	// bot that rotates it per session evades this specific check (deep-review). We deliberately
	// keep it as a key COMPONENT rather than switching to a purely server-observed key
	// (ja4|h2): l5.correlation.proxy_rotation is VerdictBot and HR-19 LONE-DENYs on it, while
	// ja4|h2 is coarse (all users of one browser version share it), so a server-only key would
	// collapse ordinary Chrome users on ≥3 /24 subnets within the TTL into one entry and
	// mass-false-DENY legitimate humans — the no-lockout constraint forbids that, and adding a
	// new, softer scored signal would touch the frozen detection path. The client fingerprint
	// supplies the per-browser entropy that keeps this FP-safe. A rotating-id bot is instead
	// caught by the SERVER-observed layers that do not depend on it: the ja4|subnet rate
	// limiter above, JA4/engine-rotation (HR-14), and IP-intel/datacenter rules.
	if client.FingerprintID != "" {
		key := client.FingerprintID + "|" + ja4
		if corr := a.corr.Observe(key, subnet, sid, now); len(corr) > 0 {
			a.store.AppendNetworkSignals(sid, corr, now)
		}
	}
	// Mid-session fingerprintId churn (client rotates fp to dodge HR-19 within one
	// cookie while hopping exits). Distinct from cross-session JA4 fan-out, which
	// would mass-false-positive ordinary Chrome users sharing one JA4 prefix.
	if sig := fingerprintChurnSignal(sid, client.FingerprintID, a); sig.ID != "" {
		a.store.AppendNetworkSignals(sid, []signals.Signal{sig}, now)
	}
	// VPN / tunnel residual (OpenVPN, WireGuard, commercial VPN): WebRTC srflx/public
	// candidates should match the TCP client IP the server sees. When they diverge, the
	// browser path (TCP, often tunnel exit) ≠ the ICE path (often the real egress) —
	// classic WebRTC leak under VPN. Server-authoritative compare; client cannot forge the
	// peer IP. Honest dual-WAN is rare enough that HR-24 CHALLENGEs rather than DENYs.
	if sig := vpnWebRTCLeakSignal(client.Advanced, clientIP(r)); sig.ID != "" {
		a.store.AppendNetworkSignals(sid, []signals.Signal{sig}, now)
	}
	if a.store.PowSolved(sid) {
		a.store.AppendNetworkSignals(sid, []signals.Signal{powSolvedSignal()}, now)
	}
}

// scoreAndStore runs the L7 engine over the assembled session and persists the
// scored report. It returns the verdict result and the scored report (the latter
// for the live-feed publish).
func (a *app) scoreAndStore(sid string, now time.Time) (signals.ScoreResult, signals.SessionReport) {
	rep, _ := a.store.Get(sid)
	res := a.engine.Score(&rep)
	a.store.StoreScored(sid, rep, now)
	return res, rep
}

// publishScored publishes the scored session to the Detection Observatory live
// feed (SoT-30 tap A), outside any shared lock. Nil (zero cost) unless the
// playground is enabled. Network residual events are published separately so
// operators can monitor TCP/proxy/VPN detections without score movement.
func (a *app) publishScored(sid string, r *http.Request, rep *signals.SessionReport) {
	if a.hub == nil {
		return
	}
	a.hub.Publish("session.scored", buildLiveEvent(sid, liveSource(r), rep))
	for _, ar := range network.AuditRecordsFromSignals(rep.Network.Signals) {
		a.hub.Publish("network.residual", map[string]any{
			"sid":       shortSessionReference(sid),
			"eventType": ar.EventType,
			"signalId":  ar.SignalID,
			"verdict":   ar.Verdict,
			"notes":     ar.Notes,
			// scoreExempt: residual never moves risk; admin Ban is the block path.
			"scoreExempt": true,
		})
	}
}

// shortSessionReference keeps the pre-existing local observatory correlation
// label without making it part of the operational log stream.
func shortSessionReference(sid string) string {
	sid = strings.Map(func(current rune) rune {
		if current < 0x20 || current == 0x7f {
			return -1
		}
		return current
	}, sid)
	if len(sid) > 8 {
		return sid[:8]
	}
	return sid
}

// writeCollectResponse writes the verdict JSON returned to the client.
func writeCollectResponse(w http.ResponseWriter, sid string, res signals.ScoreResult) {
	writeJSON(w, map[string]any{
		"sessionId":       sid,
		"riskScore":       res.RiskScore,
		"verdict":         res.Verdict,
		"hardRuleFired":   res.HardRuleFired,
		"topContributors": res.TopContributors,
		"policyVersion":   res.PolicyVersion,
	})
}
