package gate

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/gate/settings"
)

// admin_settings.go — SoT-39 P3/P3.1 write plane + dry-run/rollback.

// settingsProposeBody is the allow-listed write body (client mutationClass discarded).
type settingsProposeBody struct {
	ParentConfigVersion string                   `json:"parentConfigVersion"`
	HardRules           map[string]string        `json:"hardRules"`
	Gates               map[string]string        `json:"gates"`
	Scoring             *settings.ScoringPatch   `json:"scoring"`
	WeightMultipliers   map[string]float64       `json:"weightMultipliers"`
	NetPolicy           map[string]string        `json:"netPolicy"`
	Routes              map[string]string        `json:"routes"`
	RateLimit           *settings.RateLimitPatch `json:"rateLimit"`
	GlobalMonitor       *bool                    `json:"globalMonitor"`
	// IntegrityConfirm must be DISABLE-INTEGRITY for class C.
	IntegrityConfirm string `json:"integrityConfirm"`
	// ExpiresInSec optional; server caps B≤7d C≤24h.
	ExpiresInSec int `json:"expiresInSec"`
}

func (s *Server) adminSettingsPropose(w http.ResponseWriter, r *http.Request, op Operator) {
	if s.settingsStore == nil {
		http.Error(w, "settings store not configured (-settings-dir)", http.StatusServiceUnavailable)
		return
	}
	var body settingsProposeBody
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<18)).Decode(&body); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}
	eff := s.settingsEffectiveFresh()
	if body.ParentConfigVersion == "" || body.ParentConfigVersion != eff.ConfigVersion {
		http.Error(w, "CAS: parentConfigVersion mismatch (stale)", http.StatusConflict)
		return
	}

	ov, err := buildOverlayFromPropose(body, op.ID, eff.ConfigVersion)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if err := settings.Validate(ov); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if err := s.validateSettingsRuntime(s.settingsEffectiveCandidate(ov)); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	class := settings.Classify(eff, ov)
	ov.MutationClass = class

	if class == settings.ClassC {
		if body.IntegrityConfirm != "DISABLE-INTEGRITY" {
			http.Error(w, "class C requires integrityConfirm=DISABLE-INTEGRITY", http.StatusBadRequest)
			return
		}
	}
	exp, err := expiryForClass(class, body.ExpiresInSec, time.Now())
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	ov.ExpiresAt = exp

	if s.killSwitch.Load() && (class == settings.ClassB || class == settings.ClassC || class == settings.ClassD) {
		http.Error(w, "kill switch engaged: loosening overlays blocked", http.StatusConflict)
		return
	}

	s.sink.Emit(audit.Record{
		EventType: "settings.overlay.proposed", Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
		FailReason: "class=" + string(class) + " overlayId=" + ov.OverlayID,
	})

	if class == settings.ClassA {
		if err := s.applyOverlayCAS(ov, body.ParentConfigVersion, op.ID, ""); err != nil {
			http.Error(w, err.Error(), http.StatusConflict)
			return
		}
		writeJSON(w, map[string]any{
			"ok": true, "applied": true, "class": class, "overlayId": ov.OverlayID,
			"configVersion": s.SettingsEffective().ConfigVersion,
		})
		return
	}

	raw, _ := json.Marshal(ov)
	p := s.approvals.Create("settings.overlay", map[string]string{
		"overlayId":           ov.OverlayID,
		"parentConfigVersion": body.ParentConfigVersion,
		"mutationClass":       string(class),
		"body":                string(raw),
		"integrityConfirm":    body.IntegrityConfirm,
	}, op.ID, RoleApprover)
	s.sink.Emit(audit.Record{
		EventType: audit.EventApprovalRequested, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
		FailReason: "settings.overlay pending class=" + string(class) + " id=" + p.ID,
	})
	writeJSON(w, map[string]any{
		"ok": true, "pending": true, "approvalId": p.ID, "class": class,
		"overlayId": ov.OverlayID, "needsRole": RoleApprover,
	})
}

func (s *Server) adminSettingsOverlaysList(w http.ResponseWriter) {
	var active any
	if s.settingsStore != nil && s.settingsStore.Active() != nil {
		active = s.settingsStore.Active()
	}
	writeJSON(w, map[string]any{
		"active":    active,
		"effective": s.settingsEffectiveFresh(),
	})
}

// adminSettingsDryRun returns aggregate impact only (SoT-39 §5.6) — no session/subject data.
func (s *Server) adminSettingsDryRun(w http.ResponseWriter, r *http.Request, op Operator) {
	var body settingsProposeBody
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<18)).Decode(&body); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}
	eff := s.settingsEffectiveFresh()
	ov, err := buildOverlayFromPropose(body, op.ID, eff.ConfigVersion)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if err := settings.Validate(ov); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	class := settings.Classify(eff, ov)

	// Aggregate from recent audit: threshold band + hard-rule tags only (no re-id).
	const minN = 50
	recs := s.sink.Log().Records()
	n := 0
	var wouldAllow, wouldChallenge, wouldDeny, hrHits int
	newCh, newDn := eff.ChallengeAt, eff.DenyAt
	if ov.Scoring != nil {
		if ov.Scoring.ChallengeAt != nil {
			newCh = *ov.Scoring.ChallengeAt
		}
		if ov.Scoring.DenyAt != nil {
			newDn = *ov.Scoring.DenyAt
		}
	}
	// scan newest-first capped
	for i := len(recs) - 1; i >= 0 && n < 1000; i-- {
		rec := recs[i]
		if rec.EventType != audit.EventScoringEvaluated && rec.EventType != audit.EventEnfDeny &&
			rec.EventType != audit.EventEnfChallengeIssued && rec.EventType != audit.EventEnfAllow {
			continue
		}
		n++
		risk := float64(rec.RiskScore)
		// crude: if only threshold change, re-band risk
		band := "allow"
		if risk >= newDn {
			band = "deny"
		} else if risk >= newCh {
			band = "challenge"
		}
		// hard-rule demotion: if record had demoted HR, treat as allow-band for estimate
		demoted := false
		for _, rule := range rec.Rules {
			if ov.HardRules != nil {
				if m, ok := ov.HardRules[rule]; ok && m == settings.ModeMonitor {
					demoted = true
					hrHits++
				}
			}
		}
		if demoted {
			band = "allow"
		}
		switch band {
		case "deny":
			wouldDeny++
		case "challenge":
			wouldChallenge++
		default:
			wouldAllow++
		}
	}

	out := map[string]any{
		"class":      class,
		"n":          n,
		"minN":       minN,
		"usable":     n >= minN,
		"confidence": "approximate",
		"note":       "aggregate-only; not a freeze claim; empty store → usable=false",
	}
	if n >= minN {
		out["allowDelta"] = wouldAllow
		out["challengeDelta"] = wouldChallenge
		out["denyDelta"] = wouldDeny
		out["hardRuleDemoteHits"] = hrHits
	}
	s.sink.Emit(audit.Record{
		EventType: "settings.dry_run.completed", Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "monitor", KeyID: "k1",
		FailReason: "n=" + itoaInt(n) + " class=" + string(class),
	})
	writeJSON(w, out)
}

// adminSettingsRollback clears active overlay to empty (SoT-39 rollback).
// Classified vs current: clearing a demotion = tighten (A); clearing a tighten = loosen (B).
func (s *Server) adminSettingsRollback(w http.ResponseWriter, r *http.Request, op Operator) {
	if s.settingsStore == nil {
		http.Error(w, "settings store not configured (-settings-dir)", http.StatusServiceUnavailable)
		return
	}
	var body struct {
		ParentConfigVersion string `json:"parentConfigVersion"`
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<12)).Decode(&body); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}
	eff := s.settingsEffectiveFresh()
	if body.ParentConfigVersion == "" || body.ParentConfigVersion != eff.ConfigVersion {
		http.Error(w, "CAS: parentConfigVersion mismatch (stale)", http.StatusConflict)
		return
	}
	if eff.EmptyOverlay {
		writeJSON(w, map[string]any{"ok": true, "noop": true, "note": "already empty"})
		return
	}
	// Rollback to empty is class A if current was loosening (restoring defaults stricter for demotions),
	// class B if current was tightening (restoring weaker defaults). Use dual-control when unsure → B if any monitor HR.
	needDual := false
	if ov := s.settingsStore.Active(); ov != nil {
		if ov.MutationClass == settings.ClassA {
			needDual = true // undo tighten = loosen
		}
		if ov.MutationClass == settings.ClassB || ov.MutationClass == settings.ClassC {
			// undo demotion = tighten → A alone OK, but dual for C undo still fine as A
			needDual = false
		}
	}
	if needDual || s.killSwitch.Load() && needDual {
		// pending dual-control clear
		p := s.approvals.Create("settings.overlay.rollback", map[string]string{
			"parentConfigVersion": body.ParentConfigVersion,
			"action":              "clear",
		}, op.ID, RoleApprover)
		s.sink.Emit(audit.Record{
			EventType: audit.EventApprovalRequested, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
			TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
			FailReason: "settings rollback pending id=" + p.ID,
		})
		writeJSON(w, map[string]any{"ok": true, "pending": true, "approvalId": p.ID, "needsRole": RoleApprover})
		return
	}
	if err := s.clearOverlayCAS(body.ParentConfigVersion, op.ID, ""); err != nil {
		http.Error(w, err.Error(), http.StatusConflict)
		return
	}
	writeJSON(w, map[string]any{
		"ok": true, "rolledBack": true, "configVersion": s.SettingsEffective().ConfigVersion,
	})
}

func (s *Server) clearOverlayCAS(parent, requester, approver string) error {
	cur := s.SettingsEffective()
	if parent == "" || parent != cur.ConfigVersion {
		return s.settingsApplyRejected(errStr("CAS: parentConfigVersion mismatch (stale)"))
	}
	next := s.settingsEffectiveFor(nil)
	if err := s.validateSettingsRuntime(next); err != nil {
		return s.settingsApplyRejected(err)
	}
	if err := s.settingsStore.SetActive(nil); err != nil {
		s.recordSettingsStoreError("rollback: " + err.Error())
		return s.settingsApplyError(err)
	}
	s.syncSettingsRuntime(next)
	s.settingsStats.rolledBack.Add(1)
	fr := "settings.overlay rolled back to empty"
	if approver != "" {
		fr += " dual-control " + requester + "→" + approver
	}
	s.sink.Emit(audit.Record{
		EventType: "settings.overlay.rolled_back", Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(requester, requester)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1", FailReason: fr,
	})
	s.sink.Emit(audit.Record{
		EventType: audit.EventConfigChanged, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(requester, requester)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
		FailReason: "config_version=" + s.SettingsEffective().ConfigVersion,
	})
	return nil
}

func buildOverlayFromPropose(body settingsProposeBody, opID, parent string) (*settings.Overlay, error) {
	id, err := randOverlayID()
	if err != nil {
		return nil, err
	}
	ov := &settings.Overlay{
		SchemaVersion:       "1.0.0",
		OverlayID:           id,
		CreatedAt:           time.Now().UTC(),
		CreatedBy:           opID,
		Status:              "pending",
		ParentConfigVersion: parent,
		Scoring:             body.Scoring,
		WeightMultipliers:   body.WeightMultipliers,
		Routes:              body.Routes,
		RateLimit:           body.RateLimit,
		GlobalMonitor:       body.GlobalMonitor,
	}
	if len(body.HardRules) > 0 {
		ov.HardRules = make(map[string]settings.Mode, len(body.HardRules))
		for k, v := range body.HardRules {
			ov.HardRules[k] = settings.Mode(v)
		}
	}
	if len(body.Gates) > 0 {
		ov.Gates = make(map[string]settings.Mode, len(body.Gates))
		for k, v := range body.Gates {
			ov.Gates[k] = settings.Mode(v)
		}
	}
	if len(body.NetPolicy) > 0 {
		ov.NetPolicy = make(map[string]settings.Mode, len(body.NetPolicy))
		for k, v := range body.NetPolicy {
			ov.NetPolicy[k] = settings.Mode(v)
		}
	}
	return ov, nil
}

func expiryForClass(class settings.MutationClass, wantSec int, now time.Time) (*time.Time, error) {
	switch class {
	case settings.ClassA:
		if wantSec <= 0 {
			return nil, nil
		}
		t := now.Add(time.Duration(wantSec) * time.Second)
		return &t, nil
	case settings.ClassB, settings.ClassD:
		sec := wantSec
		if sec <= 0 {
			sec = 7 * 24 * 3600
		}
		if sec > 7*24*3600 {
			return nil, errStr("class B/D expiresInSec max 7d")
		}
		t := now.Add(time.Duration(sec) * time.Second)
		return &t, nil
	case settings.ClassC:
		sec := wantSec
		if sec <= 0 {
			sec = 24 * 3600
		}
		if sec > 24*3600 {
			return nil, errStr("class C expiresInSec max 24h")
		}
		t := now.Add(time.Duration(sec) * time.Second)
		return &t, nil
	default:
		return nil, nil
	}
}

type errStr string

func (e errStr) Error() string { return string(e) }

func randOverlayID() (string, error) {
	b := make([]byte, 8)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return "ovl_" + hex.EncodeToString(b), nil
}

func (s *Server) applyOverlayCAS(ov *settings.Overlay, parent string, requester, approver string) error {
	if s.settingsStore == nil {
		return s.settingsApplyRejected(errStr("settings store not configured"))
	}
	cur := s.SettingsEffective()
	if parent == "" || parent != cur.ConfigVersion {
		return s.settingsApplyRejected(errStr("CAS: parentConfigVersion mismatch (stale)"))
	}
	if err := settings.Validate(ov); err != nil {
		return s.settingsApplyRejected(err)
	}
	class := settings.Classify(cur, ov)
	ov.MutationClass = class
	if class == settings.ClassC && ov.ExpiresAt == nil {
		return s.settingsApplyRejected(errStr("class C requires expiresAt"))
	}
	if ov.ExpiresAt != nil && !ov.ExpiresAt.After(time.Now()) {
		return s.settingsApplyRejected(errStr("overlay already expired"))
	}
	ov.Status = "active"
	ov.ApprovedBy = approver
	next := s.settingsEffectiveCandidate(ov)
	if err := s.validateSettingsRuntime(next); err != nil {
		return s.settingsApplyRejected(err)
	}
	if err := s.settingsStore.SetActive(ov); err != nil {
		s.recordSettingsStoreError("apply: " + err.Error())
		return s.settingsApplyError(err)
	}
	s.syncSettingsRuntime(next)
	s.settingsStats.applied.Add(1)
	fr := "settings.overlay applied class=" + string(class) + " id=" + ov.OverlayID
	if approver != "" {
		fr += " dual-control " + requester + "→" + approver
	}
	s.sink.Emit(audit.Record{
		EventType: "settings.overlay.applied", Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(requester, requester)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1", FailReason: fr,
	})
	s.sink.Emit(audit.Record{
		EventType: audit.EventConfigChanged, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(requester, requester)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
		FailReason: "config_version=" + s.SettingsEffective().ConfigVersion,
	})
	return nil
}

func (s *Server) commitSettingsOverlay(p PendingAction, approver Operator) error {
	if p.Requester == approver.ID {
		return errStr("approver must differ from requester")
	}
	raw := p.Params["body"]
	if raw == "" {
		return errStr("missing overlay body")
	}
	var ov settings.Overlay
	if err := json.Unmarshal([]byte(raw), &ov); err != nil {
		return err
	}
	parent := p.Params["parentConfigVersion"]
	if class := p.Params["mutationClass"]; class == string(settings.ClassC) {
		if p.Params["integrityConfirm"] != "DISABLE-INTEGRITY" {
			return errStr("class C requires integrityConfirm")
		}
	}
	return s.applyOverlayCAS(&ov, parent, p.Requester, approver.ID)
}

func (s *Server) expireActiveOverlayIfNeeded() {
	if s.settingsStore == nil || s.killSwitch.Load() {
		return
	}
	ov := s.settingsStore.Active()
	if ov == nil || ov.ExpiresAt == nil {
		return
	}
	if time.Now().After(*ov.ExpiresAt) {
		next := s.settingsEffectiveFor(nil)
		if err := s.settingsStore.SetActive(nil); err != nil {
			s.recordSettingsStoreError("expire: " + err.Error())
			return
		}
		s.syncSettingsRuntime(next)
		s.sink.Emit(audit.Record{
			EventType: "settings.overlay.expired", Actor: audit.Actor{Kind: "system"},
			TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
			FailReason: "overlayId=" + ov.OverlayID,
		})
	}
}

func (s *Server) settingsEffectiveFresh() settings.Effective {
	s.expireActiveOverlayIfNeeded()
	return s.SettingsEffective()
}

func adminSettingsEffectiveBody(s *Server) map[string]any {
	eff := s.settingsEffectiveFresh()
	out := map[string]any{
		"effective":     eff,
		"storeAttached": s.settingsStore != nil,
	}
	if s.settingsStore != nil && s.settingsStore.LoadError() != nil {
		out["storeWarning"] = s.settingsStore.LoadError().Error()
	}
	return out
}
