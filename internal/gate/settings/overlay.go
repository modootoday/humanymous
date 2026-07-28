// Package settings implements SoT-39 RuntimeOverlay: validation, effective
// resolution, and file persistence. P1 = read path + store; P2+ wires Engine.
//
// Empty overlay (nil / zero) MUST resolve to pre-Settings code defaults so
// freeze goldens and the red catalog stay behavior-identical.
package settings

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"sort"
	"time"
)

// Mode is enforce|monitor|shadow|off (SoT-39 §2.1). Engine HRs use enforce|monitor only.
type Mode string

const (
	ModeEnforce Mode = "enforce"
	ModeMonitor Mode = "monitor"
	ModeShadow  Mode = "shadow"
	ModeOff     Mode = "off"
)

// MutationClass is server-computed (never trust client).
type MutationClass string

const (
	ClassA MutationClass = "A" // tighten
	ClassB MutationClass = "B" // loosen non-integrity
	ClassC MutationClass = "C" // neutralize / integrity
	ClassD MutationClass = "D" // route weaken
)

// Overlay is a versioned operator delta (SoT-39 §4.1).
type Overlay struct {
	SchemaVersion       string             `json:"schemaVersion"`
	OverlayID           string             `json:"overlayId"`
	CreatedAt           time.Time          `json:"createdAt"`
	CreatedBy           string             `json:"createdBy,omitempty"`
	ApprovedBy          string             `json:"approvedBy,omitempty"`
	Status              string             `json:"status"` // pending|active|rolled_back|expired
	ExpiresAt           *time.Time         `json:"expiresAt,omitempty"`
	ParentConfigVersion string             `json:"parentConfigVersion,omitempty"`
	MutationClass       MutationClass      `json:"mutationClass,omitempty"` // server-set
	Gates               map[string]Mode    `json:"gates,omitempty"`
	HardRules           map[string]Mode    `json:"hardRules,omitempty"`
	Scoring             *ScoringPatch      `json:"scoring,omitempty"`
	WeightMultipliers   map[string]float64 `json:"weightMultipliers,omitempty"`
	NetPolicy           map[string]Mode    `json:"netPolicy,omitempty"`
	Routes              map[string]string  `json:"routes,omitempty"`
	RateLimit           *RateLimitPatch    `json:"rateLimit,omitempty"`
	GlobalMonitor       *bool              `json:"globalMonitor,omitempty"`
}

// ScoringPatch overrides SoT-05 Policy knobs (nil fields = no override).
type ScoringPatch struct {
	ChallengeAt *float64 `json:"challengeAt,omitempty"`
	DenyAt      *float64 `json:"denyAt,omitempty"`
	LayerCap    *float64 `json:"layerCap,omitempty"`
}

// RateLimitPatch overrides SoT-27 thresholds.
type RateLimitPatch struct {
	WindowSec *int `json:"windowSec,omitempty"`
	Soft      *int `json:"soft,omitempty"`
	Hard      *int `json:"hard,omitempty"`
}

// ScoringDefaults are code defaults (SoT-05 policyVersion 1.0.0).
func ScoringDefaults() (challengeAt, denyAt, layerCap float64) {
	return 30, 70, 60
}

// IntegrityCriticalEngineHRs demote enforce→monitor = class C (SoT-39 §3.2.2).
var IntegrityCriticalEngineHRs = map[string]struct{}{
	"HR-1": {}, "HR-2": {}, "HR-16": {}, "HR-18": {}, "HR-19": {},
}

// IntegrityCriticalGates cannot be off; demote below enforce = class C.
var IntegrityCriticalGates = map[string]struct{}{
	"gate.smuggle":       {},
	"gate.spoof_header":  {},
	"gate.verdict_token": {},
}

// KnownGates is the v1 module catalog.
var KnownGates = []string{
	"gate.smuggle", "gate.spoof_header", "gate.ban", "gate.verdict_token",
	"gate.recon_sweep", "gate.inject", "gate.attest_floor",
}

// NetPolicyClasses known residual classes (SoT-39 §3.5).
var NetPolicyClasses = []string{
	"net.proxy.hop", "net.proxy.anon", "net.proxy.spoof", "net.vpn",
	"net.tor", "net.tcp", "net.correlation", "net.h2.spoof", "net.header.order", "net.tls.pq",
	"net.tls.alps",
}

// CanonicalJSON returns stable JSON for signing / config_version (sorted maps).
func (o *Overlay) CanonicalJSON() ([]byte, error) {
	if o == nil {
		return []byte("{}"), nil
	}
	// Re-encode via map for stable key order on nested maps.
	type wire struct {
		SchemaVersion       string             `json:"schemaVersion"`
		OverlayID           string             `json:"overlayId"`
		Status              string             `json:"status"`
		ParentConfigVersion string             `json:"parentConfigVersion,omitempty"`
		MutationClass       string             `json:"mutationClass,omitempty"`
		Gates               map[string]string  `json:"gates,omitempty"`
		HardRules           map[string]string  `json:"hardRules,omitempty"`
		Scoring             *ScoringPatch      `json:"scoring,omitempty"`
		WeightMultipliers   map[string]float64 `json:"weightMultipliers,omitempty"`
		NetPolicy           map[string]string  `json:"netPolicy,omitempty"`
		Routes              map[string]string  `json:"routes,omitempty"`
		RateLimit           *RateLimitPatch    `json:"rateLimit,omitempty"`
		GlobalMonitor       *bool              `json:"globalMonitor,omitempty"`
	}
	w := wire{
		SchemaVersion:       o.SchemaVersion,
		OverlayID:           o.OverlayID,
		Status:              o.Status,
		ParentConfigVersion: o.ParentConfigVersion,
		MutationClass:       string(o.MutationClass),
		Scoring:             o.Scoring,
		RateLimit:           o.RateLimit,
		GlobalMonitor:       o.GlobalMonitor,
	}
	if len(o.Gates) > 0 {
		w.Gates = sortedModeMap(o.Gates)
	}
	if len(o.HardRules) > 0 {
		w.HardRules = sortedModeMap(o.HardRules)
	}
	if len(o.NetPolicy) > 0 {
		w.NetPolicy = sortedModeMap(o.NetPolicy)
	}
	if len(o.Routes) > 0 {
		w.Routes = sortedStringMap(o.Routes)
	}
	if len(o.WeightMultipliers) > 0 {
		w.WeightMultipliers = sortedFloatMap(o.WeightMultipliers)
	}
	return json.Marshal(w)
}

func sortedModeMap(m map[string]Mode) map[string]string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := make(map[string]string, len(m))
	for _, k := range keys {
		out[k] = string(m[k])
	}
	return out
}

func sortedStringMap(m map[string]string) map[string]string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := make(map[string]string, len(m))
	for _, k := range keys {
		out[k] = m[k]
	}
	return out
}

func sortedFloatMap(m map[string]float64) map[string]float64 {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := make(map[string]float64, len(m))
	for _, k := range keys {
		out[k] = m[k]
	}
	return out
}

// OverlayDigest is a short HMAC of canonical overlay body for config_version.
func OverlayDigest(o *Overlay, key []byte) string {
	if o == nil || o.OverlayID == "" {
		return ""
	}
	body, err := o.CanonicalJSON()
	if err != nil {
		return "err"
	}
	h := hmac.New(sha256.New, key)
	_, _ = h.Write(body)
	return hex.EncodeToString(h.Sum(nil)[:6])
}

// HashConfigVersion builds cfg-… including overlay digest (SoT-39 §4.4).
func HashConfigVersion(key []byte, globalMonitor, killSwitch bool, routes map[string]string, windowSec, soft, hard int, overlay *Overlay) string {
	h := hmac.New(sha256.New, key)
	fmt.Fprintf(h, "monitor=%v|kill=%v", globalMonitor, killSwitch)
	prefixes := make([]string, 0, len(routes))
	for p := range routes {
		prefixes = append(prefixes, p)
	}
	sort.Strings(prefixes)
	for _, p := range prefixes {
		fmt.Fprintf(h, "|%s=%s", p, routes[p])
	}
	fmt.Fprintf(h, "|rl=%d/%d/%d", windowSec, soft, hard)
	if d := OverlayDigest(overlay, key); d != "" {
		fmt.Fprintf(h, "|ovl=%s|id=%s", d, overlay.OverlayID)
	}
	return "cfg-" + hex.EncodeToString(h.Sum(nil)[:6])
}
