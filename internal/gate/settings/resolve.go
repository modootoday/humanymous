package settings

// Effective is the resolved posture operators and Engine read (SoT-39 §2).
type Effective struct {
	ConfigVersion string             `json:"configVersion"`
	GlobalMonitor bool               `json:"globalMonitor"`
	KillSwitch    bool               `json:"killSwitch"`
	Gates         map[string]Mode    `json:"gates"`
	HardRules     map[string]Mode    `json:"hardRules"` // only overrides; empty = all enforce
	ChallengeAt   float64            `json:"challengeAt"`
	DenyAt        float64            `json:"denyAt"`
	LayerCap      float64            `json:"layerCap"`
	Weights       map[string]float64 `json:"weightMultipliers,omitempty"`
	NetPolicy     map[string]Mode    `json:"netPolicy,omitempty"`
	Routes        map[string]string  `json:"routes"`
	RateWindowSec int                `json:"rateWindowSec"`
	RateSoft      int                `json:"rateSoft"`
	RateHard      int                `json:"rateHard"`
	OverlayID     string             `json:"overlayId,omitempty"`
	OverlayStatus string             `json:"overlayStatus,omitempty"`
	EmptyOverlay  bool               `json:"emptyOverlay"`
}

// BootInput is the process boot config slice Settings needs (no import cycle with gate.Config).
type BootInput struct {
	GlobalMonitor bool
	KillSwitch    bool
	Routes        map[string]string
	RateWindowSec int
	RateSoft      int
	RateHard      int
	HMACKey       []byte
}

// Resolve builds Effective from boot + optional active overlay (SoT-39 §2.2).
// Empty/nil overlay ⇒ pre-Settings defaults (EmptyOverlay true).
func Resolve(boot BootInput, active *Overlay) Effective {
	ch, dn, lc := ScoringDefaults()
	eff := Effective{
		GlobalMonitor: boot.GlobalMonitor,
		KillSwitch:    boot.KillSwitch,
		Gates:         defaultGates(),
		HardRules:     map[string]Mode{},
		ChallengeAt:   ch,
		DenyAt:        dn,
		LayerCap:      lc,
		Routes:        copyStrMap(boot.Routes),
		RateWindowSec: boot.RateWindowSec,
		RateSoft:      boot.RateSoft,
		RateHard:      boot.RateHard,
		EmptyOverlay:  true,
	}
	if eff.Routes == nil {
		eff.Routes = map[string]string{}
	}
	if active != nil && (active.Status == "active" || active.Status == "") {
		eff.EmptyOverlay = false
		eff.OverlayID = active.OverlayID
		eff.OverlayStatus = active.Status
		if active.Status == "" {
			eff.OverlayStatus = "active"
		}
		for k, m := range active.Gates {
			eff.Gates[k] = m
		}
		for k, m := range active.HardRules {
			eff.HardRules[k] = m
		}
		if active.Scoring != nil {
			if active.Scoring.ChallengeAt != nil {
				eff.ChallengeAt = *active.Scoring.ChallengeAt
			}
			if active.Scoring.DenyAt != nil {
				eff.DenyAt = *active.Scoring.DenyAt
			}
			if active.Scoring.LayerCap != nil {
				eff.LayerCap = *active.Scoring.LayerCap
			}
		}
		if len(active.WeightMultipliers) > 0 {
			eff.Weights = copyFloatMap(active.WeightMultipliers)
		}
		if len(active.NetPolicy) > 0 {
			eff.NetPolicy = copyModeMap(active.NetPolicy)
		}
		for k, v := range active.Routes {
			eff.Routes[k] = v
		}
		if active.RateLimit != nil {
			if active.RateLimit.WindowSec != nil {
				eff.RateWindowSec = *active.RateLimit.WindowSec
			}
			if active.RateLimit.Soft != nil {
				eff.RateSoft = *active.RateLimit.Soft
			}
			if active.RateLimit.Hard != nil {
				eff.RateHard = *active.RateLimit.Hard
			}
		}
		if active.GlobalMonitor != nil {
			eff.GlobalMonitor = *active.GlobalMonitor
		}
	}
	// Kill switch always wins: force monitor posture flag (enforcement suppressed).
	if boot.KillSwitch {
		eff.GlobalMonitor = true
	}
	eff.ConfigVersion = HashConfigVersion(boot.HMACKey, eff.GlobalMonitor, boot.KillSwitch,
		eff.Routes, eff.RateWindowSec, eff.RateSoft, eff.RateHard, active)
	return eff
}

// HRMode returns enforce if no override (empty overlay / missing key).
func (e Effective) HRMode(id string) Mode {
	if m, ok := e.HardRules[id]; ok {
		return m
	}
	return ModeEnforce
}

// GateMode returns enforce if unknown.
func (e Effective) GateMode(id string) Mode {
	if m, ok := e.Gates[id]; ok {
		return m
	}
	return ModeEnforce
}

func defaultGates() map[string]Mode {
	m := make(map[string]Mode, len(KnownGates))
	for _, g := range KnownGates {
		m[g] = ModeEnforce
	}
	return m
}

func copyStrMap(in map[string]string) map[string]string {
	if in == nil {
		return nil
	}
	out := make(map[string]string, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}

func copyFloatMap(in map[string]float64) map[string]float64 {
	out := make(map[string]float64, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}

func copyModeMap(in map[string]Mode) map[string]Mode {
	out := make(map[string]Mode, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}
