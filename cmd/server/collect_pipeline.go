package main

import (
	"encoding/json"
	"net/http"
	"strconv"
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
	obs := network.Observation{
		Hello:             a.reg.Hello(r.RemoteAddr),
		H2:                a.reg.H2(r.RemoteAddr),
		Header:            reqToHeaderInfo(r),
		IsDatacenterIP:    isDatacenterIP(clientIP(r)),
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
	if client.FingerprintID != "" {
		key := client.FingerprintID + "|" + ja4
		if corr := a.corr.Observe(key, subnet, sid, now); len(corr) > 0 {
			a.store.AppendNetworkSignals(sid, corr, now)
		}
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
	if a.logEnabled { // PLAN-07 R11: one structured verdict line, opt-in, no raw IP/UA
		top := ""
		if len(res.TopContributors) > 0 {
			top = res.TopContributors[0].ID
		}
		a.log.Info("scored", "sid", shortSID(sid), "verdict", res.Verdict,
			"risk", res.RiskScore, "hardRule", res.HardRuleFired, "top", top)
	}
	return res, rep
}

// publishScored publishes the scored session to the Detection Observatory live
// feed (SoT-30 tap A), outside any shared lock. Nil (zero cost) unless the
// playground is enabled.
func (a *app) publishScored(sid string, r *http.Request, rep *signals.SessionReport) {
	if a.hub != nil {
		a.hub.Publish("session.scored", buildLiveEvent(sid, liveSource(r), rep))
	}
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
