package gate

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/network"
	"github.com/modootoday/humanymous/internal/signals"
)

// control_collect.go decomposes the control-plane /collect beacon handler into one
// method per DOMAIN (SoT-19 step 5): body parse, beacon-replay defense (HR-29),
// scoring, the sticky verdict, the verdict trust token (HR-28), and the audit
// record. handleCollect (control.go) orchestrates these in order.

// parseControlClient unmarshals the beacon body and binds the session id.
func parseControlClient(w http.ResponseWriter, body []byte, sid string) (signals.ClientReport, bool) {
	var client signals.ClientReport
	if err := json.Unmarshal(body, &client); err != nil {
		http.Error(w, "bad body", http.StatusBadRequest)
		return client, false
	}
	client.SessionID = sid
	return client, true
}

// rejectBeaconReplay enforces the single-use beacon nonce (SoT-21 §4, HR-29): a
// replayed beacon carries an already-consumed nonce -> audit + 409. Returns true
// when the request was rejected (the caller must stop).
func (c *ControlPlane) rejectBeaconReplay(w http.ResponseWriter, r *http.Request, sid string, now time.Time) bool {
	nonce := r.Header.Get("X-Hmn-Nonce")
	if nonce == "" || c.nonces.Use(nonce, bindKey(r), now) {
		return false
	}
	c.sink.Emit(audit.Record{
		EventType: audit.EventBeaconReplay, Actor: audit.Actor{Kind: "subject", IDPsn: c.pseudo(sid, sid)},
		TenantID: "control", RouteClass: "control", Verdict: string(VerdictDeny),
		Rules: []string{"HR-29"}, Action: "block", Mode: "enforce", FailReason: "beacon nonce replay", KeyID: "k1",
	})
	http.Error(w, "replay rejected", http.StatusConflict)
	return true
}

// scoreBeacon merges the header-derived network observation with the client
// report, scores the session (L7), and persists it. Returns the verdict result.
func (c *ControlPlane) scoreBeacon(sid string, r *http.Request, client signals.ClientReport, now time.Time) signals.ScoreResult {
	obs := network.Observation{Header: headerInfo(r), IsDatacenterIP: false}
	c.store.MergeNetwork(sid, network.Build(obs), now)
	c.store.MergeClient(sid, client, now)
	if lbl := r.URL.Query().Get("label"); lbl != "" {
		c.store.SetLabel(sid, lbl, now)
	}
	rep, _ := c.store.Get(sid)
	res := c.engine.Score(&rep)
	c.store.StoreScored(sid, rep, now)
	return res
}

// updateStickyVerdict updates the sticky verdict the edge gate enforces.
func (c *ControlPlane) updateStickyVerdict(sid string, res signals.ScoreResult, now time.Time) {
	c.verdicts.Set(sid, stickyVerdict{
		verdict: mapVerdict(res.Verdict),
		risk:    res.RiskScore,
		rule:    res.HardRuleFired,
		top:     topSignals(res.TopContributors),
		updated: now,
	})
}

// issueVerdictTokenOnAllow issues a fingerprint-bound verdict trust token on ALLOW
// (SoT-21 §3, HR-28) so the edge fast-path can trust this client without
// re-scoring; a token replayed from a different fingerprint is rejected.
func (c *ControlPlane) issueVerdictTokenOnAllow(w http.ResponseWriter, r *http.Request, sid string, res signals.ScoreResult, now time.Time) {
	if res.Verdict != "ALLOW" || len(c.tokenKey) == 0 {
		return
	}
	tok := issueVerdictToken(c.tokenKey, sid, tokenBind(r), c.tokEpoch(), now.Add(30*time.Minute))
	http.SetCookie(w, &http.Cookie{Name: verdictCookie, Value: tok, Path: "/", HttpOnly: true, SameSite: http.SameSiteLaxMode})
	c.sink.Emit(audit.Record{
		EventType: audit.EventTokenIssued, Actor: audit.Actor{Kind: "subject", IDPsn: c.pseudo(sid, sid)},
		TenantID: "control", RouteClass: "control", Verdict: "allow", Mode: "enforce", KeyID: "k1",
	})
}

// linkAndAuditScoring records the handle->subject reverse mapping (SoT-28 §6) so an
// operator can request erasure by pseudonym, and audits the scoring evaluation
// (informational; enforcement happens at the edge).
func (c *ControlPlane) linkAndAuditScoring(sid string, r *http.Request, res signals.ScoreResult) {
	if c.vault != nil {
		c.vault.Link(c.pseudo(sid, sid), sid)
	}
	c.sink.Emit(audit.Record{
		EventType:  audit.EventScoringEvaluated,
		Actor:      audit.Actor{Kind: "subject", IDPsn: c.pseudo(sid, clientIP(r))},
		TenantID:   "control",
		SessionPsn: c.pseudo(sid, sid),
		RouteClass: "control",
		Verdict:    string(mapVerdict(res.Verdict)),
		RiskScore:  int(res.RiskScore),
		Rules:      ruleList(res.HardRuleFired),
		TopSignals: topSignals(res.TopContributors),
		Mode:       "enforce",
		KeyID:      "k1",
	})
}

// writeControlVerdict writes the beacon verdict JSON returned to the client.
func writeControlVerdict(w http.ResponseWriter, sid string, res signals.ScoreResult) {
	writeJSON(w, map[string]any{
		"sessionId":     sid,
		"riskScore":     res.RiskScore,
		"verdict":       res.Verdict,
		"hardRuleFired": res.HardRuleFired,
	})
}
