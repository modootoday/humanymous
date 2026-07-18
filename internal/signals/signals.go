// Package signals defines the canonical signal/report schema shared by the
// WASM client detector (cmd/wasm) and the Go backend (cmd/server).
//
// This package MUST stay free of platform-specific imports (no crypto/tls,
// net/http, or syscall/js) so it compiles for both GOARCH=wasm and the server.
//
// Normative reference: sots/00-overview.md §3 (Canonical Schema).
package signals

import "time"

// Layer identifies the detection layer a signal belongs to (SoT-00 §2).
type Layer string

const (
	LayerStatic      Layer = "L1" // static client (webdriver, headless tells)
	LayerFingerprint Layer = "L2" // canvas/webgl/audio/screen/hardware
	LayerIntegrity   Layer = "L3" // client tamper/integrity checks
	LayerBehavioral  Layer = "L4" // mouse/keystroke/scroll, isTrusted
	LayerNetwork     Layer = "L5" // TLS/HTTP2/header (server captured)
	LayerConsistency Layer = "L6" // cross-layer consistency
	LayerScoring     Layer = "L7" // scoring/decision
)

// Verdict is a per-signal qualitative judgement.
type Verdict string

const (
	VerdictOK         Verdict = "OK"         // matches human expectation
	VerdictSuspicious Verdict = "SUSPICIOUS" // possible bot, not conclusive
	VerdictBot        Verdict = "BOT"        // strong automation evidence
	VerdictUnknown    Verdict = "UNKNOWN"    // not collected / unsupported
)

// Source identifies where a signal was collected.
type Source string

const (
	SourceJS     Source = "js"
	SourceWASM   Source = "wasm"
	SourceServer Source = "server"
)

// Signal is one atomic observation (SoT-00 §3.1).
type Signal struct {
	ID            string  `json:"id"`    // dotted id: L{n}.{group}.{item}
	Layer         Layer   `json:"layer"` // L1..L7
	Value         any     `json:"value"` // observed value
	ExpectedHuman any     `json:"expectedHuman,omitempty"`
	Verdict       Verdict `json:"verdict"`    // OK|SUSPICIOUS|BOT|UNKNOWN
	Weight        float64 `json:"weight"`     // max risk contribution 0..100
	Score         float64 `json:"score"`      // actual risk contribution 0..weight
	Confidence    float64 `json:"confidence"` // observation confidence 0..1
	Collected     Source  `json:"collected"`  // js|wasm|server
	Notes         string  `json:"notes,omitempty"`
}

// CrossCheck is an L6 consistency evaluation between signals/layers (SoT-00 §3.3).
type CrossCheck struct {
	ID         string   `json:"id"`
	Inputs     []string `json:"inputs"`
	Claim      string   `json:"claim"`
	Observed   string   `json:"observed"`
	Consistent bool     `json:"consistent"`
	Weight     float64  `json:"weight"`
	Score      float64  `json:"score"`
	Notes      string   `json:"notes,omitempty"`
}

// UAClientHints holds parsed User-Agent Client Hints (low + high entropy).
type UAClientHints struct {
	Brands          []UABrand `json:"brands,omitempty"`
	FullVersionList []UABrand `json:"fullVersionList,omitempty"`
	Mobile          bool      `json:"mobile"`
	Platform        string    `json:"platform,omitempty"`
	PlatformVersion string    `json:"platformVersion,omitempty"`
	Architecture    string    `json:"architecture,omitempty"`
	Bitness         string    `json:"bitness,omitempty"`
	Model           string    `json:"model,omitempty"`
}

// UABrand is a single brand/version pair from Client Hints.
type UABrand struct {
	Brand   string `json:"brand"`
	Version string `json:"version"`
}

// ClientReport is the client-collected portion (L1-L4) sent to /api/collect.
type ClientReport struct {
	SessionID     string        `json:"sessionId"`
	UserAgent     string        `json:"userAgent"`
	UAClientHints UAClientHints `json:"uaClientHints"`
	Signals       []Signal      `json:"signals"`
	// Behavior carries reduced behavioral feature vectors (L4). Raw event
	// streams are summarised client-side; see sots/03-behavioral-biometrics.md.
	Behavior BehaviorSummary `json:"behavior"`
	// Environment carries privacy/adblock probe results (SoT-09).
	Environment Environment `json:"environment"`
	// Guard carries script-injection / eval guard results (SoT-11).
	Guard Guard `json:"guard"`
	// Advanced carries the advanced fingerprint surfaces (SoT-14): WebRTC,
	// voices, WebGPU, media capabilities, connection, timezone, matchMedia.
	Advanced Advanced `json:"advanced"`
	// FingerprintID is a stable composite hash of the client's hard-to-change
	// surfaces (canvas/audio/webgl/fonts/timezone) used for cross-session
	// correlation to unmask residential-proxy rotation (SoT-15).
	FingerprintID string `json:"fingerprintId"`
	// EngineVersion of the WASM detector that produced this report.
	EngineVersion string `json:"engineVersion"`
}

// Advanced holds advanced fingerprint surfaces (SoT-14). Absences are weak
// tells (privacy browsers / OS variation); the power is in cross-surface
// consistency, evaluated in internal/scoring/advanced.go.
type Advanced struct {
	Probed bool `json:"probed"`
	// WebRTC / media devices.
	MediaDeviceCount int  `json:"mediaDeviceCount"`
	HasAudioInput    bool `json:"hasAudioInput"`
	HasVideoInput    bool `json:"hasVideoInput"`
	// Speech synthesis.
	VoiceCount int `json:"voiceCount"`
	// EME / Widevine DRM.
	WidevineSupported bool `json:"widevineSupported"`
	// WebGPU.
	WebGPUPresent  bool   `json:"webgpuPresent"`
	WebGPUVendor   string `json:"webgpuVendor"`
	WebGPUFallback bool   `json:"webgpuFallback"`
	WebGLVendor    string `json:"webglVendor"`
	// Audio.
	AudioSampleRate float64 `json:"audioSampleRate"`
	// Network Information.
	ConnectionPresent bool    `json:"connectionPresent"`
	ConnectionRTT     float64 `json:"connectionRtt"`
	// Battery.
	BatteryPresent  bool    `json:"batteryPresent"`
	BatteryLevel    float64 `json:"batteryLevel"`
	BatteryCharging bool    `json:"batteryCharging"`
	// Timezone / locale.
	TimezoneIANA   string `json:"timezoneIana"`
	TimezoneOffset int    `json:"timezoneOffsetMin"`
	Language       string `json:"language"`
	// matchMedia display profile.
	ColorGamut     string `json:"colorGamut"`
	DynamicRange   string `json:"dynamicRange"`
	PointerCoarse  bool   `json:"pointerCoarse"`
	MaxTouchPoints int    `json:"maxTouchPoints"`
	// Chrome-only memory API presence + new-headless tell.
	PerfMemoryPresent bool `json:"perfMemoryPresent"`
	MeasureMemThrows  bool `json:"measureMemThrows"`
	// UA hints for consistency (mobile claim).
	MobileUA bool `json:"mobileUA"`
}

// Guard holds runtime-hardening probe results from injector.js (SoT-11).
type Guard struct {
	Reported        bool `json:"reported"`
	EvalUsed        int  `json:"evalUsed"`
	FuncCtor        int  `json:"funcCtor"`
	ScriptInjected  int  `json:"scriptInjected"`
	ProtoPolluted   bool `json:"protoPolluted"`
	NativeHooked    bool `json:"nativeHooked"`
	ConsoleDisabled bool `json:"consoleDisabled"`
}

// Environment holds ad-block / tracking-protection probe results (SoT-09).
// These are treated as neutral-to-human (privacy users are common); they feed
// consistency cross-checks and FP mitigation, never a standalone bot verdict.
type Environment struct {
	Probed             bool   `json:"probed"`             // probe actually ran (liveness)
	AdBlock            bool   `json:"adBlock"`            // bait ad element/request blocked
	TrackingProtection bool   `json:"trackingProtection"` // known tracker request blocked
	DoNotTrack         string `json:"doNotTrack"`         // navigator.doNotTrack ("1"|"0"|"")
	GPC                bool   `json:"gpc"`                // navigator.globalPrivacyControl
	CollectorBlocked   bool   `json:"collectorBlocked"`   // our own collector/RIT resource was blocked
	BraveShields       bool   `json:"braveShields"`       // navigator.brave present (Brave)
}

// BehaviorSummary is the reduced feature vector from the behavioral collector.
type BehaviorSummary struct {
	Mouse     MouseFeatures `json:"mouse"`
	Key       KeyFeatures   `json:"key"`
	Events    EventFeatures `json:"events"`
	DurationS float64       `json:"durationS"` // observation window seconds
}

// MouseFeatures summarises pointer movement (SoT-03 §1).
type MouseFeatures struct {
	Samples          int     `json:"samples"`
	MeanCurvature    float64 `json:"meanCurvature"`    // 0=straight lines
	VelocityStdDev   float64 `json:"velocityStdDev"`   // human: nonzero variance
	AccelEntropy     float64 `json:"accelEntropy"`     // shannon entropy of accel bins
	StraightLineFrac float64 `json:"straightLineFrac"` // frac of near-perfect lines
	PauseCount       int     `json:"pauseCount"`
	MaxTeleportPx    float64 `json:"maxTeleportPx"` // largest instantaneous jump
	MeanJerk         float64 `json:"meanJerk"`      // higher-order smoothness
	// CoalescedRatio is the mean getCoalescedEvents() count per pointermove.
	// Real mice/trackpads emit many coalesced sub-events at high frequency;
	// synthetic (CDP/OS-injected) input emits ~1 (SoT-14). A movement-rich
	// session with CoalescedRatio≈1 is a strong synthetic-input tell.
	CoalescedRatio float64 `json:"coalescedRatio"`
}

// KeyFeatures summarises keystroke dynamics (SoT-03 §2).
type KeyFeatures struct {
	Keystrokes    int     `json:"keystrokes"`
	MeanDwellMs   float64 `json:"meanDwellMs"`   // key press->release
	DwellStdDevMs float64 `json:"dwellStdDevMs"` // human: nonzero variance
	MeanFlightMs  float64 `json:"meanFlightMs"`  // release->next press
	FlightStdDev  float64 `json:"flightStdDevMs"`
	PasteEvents   int     `json:"pasteEvents"`
	ZeroDwellFrac float64 `json:"zeroDwellFrac"` // synthetic keys often 0
}

// EventFeatures captures trust/dispatch artifacts + AI-agent cadence (SoT-03 §5,
// SoT-16). Per FP-Agent (arXiv 2605.01247), behavioral cadence separates AI
// browsing agents from humans almost perfectly.
type EventFeatures struct {
	TotalEvents    int     `json:"totalEvents"`
	UntrustedFrac  float64 `json:"untrustedFrac"`  // isTrusted===false ratio
	NoMoveClicks   int     `json:"noMoveClicks"`   // clicks with no preceding move
	SyntheticFlags int     `json:"syntheticFlags"` // CDP dispatch artifacts seen
	ClickCount     int     `json:"clickCount"`     // pointerdown count
	// LLM inference-loop cadence (SoT-16): long "thinking" gaps punctuating
	// bursts of near-instant, low-variance actions.
	LongGapCount  int     `json:"longGapCount"`  // inter-event gaps > 1.5s
	BurstVarMs    float64 `json:"burstVarMs"`    // variance of intra-burst gaps
	MinReactionMs float64 `json:"minReactionMs"` // fastest action after a long gap
}

// ScoreResult is the L7 output (SoT-00 §3.2 scoring).
type ScoreResult struct {
	RiskScore       float64       `json:"riskScore"` // 0..100
	Verdict         string        `json:"verdict"`   // ALLOW|CHALLENGE|DENY
	HardRuleFired   string        `json:"hardRuleFired,omitempty"`
	TopContributors []Contributor `json:"topContributors"`
	PolicyVersion   string        `json:"policyVersion"`
}

// Contributor is one signal's contribution to the final score.
type Contributor struct {
	ID    string  `json:"id"`
	Score float64 `json:"score"`
}

// NetworkReport is the server-captured portion (L5).
type NetworkReport struct {
	JA3     string `json:"ja3"`
	JA3Text string `json:"ja3Text"`
	JA4     string `json:"ja4"`
	JA4Raw  string `json:"ja4Raw,omitempty"`
	// JA4Engine / H2Engine are the browser-engine families the network layer
	// resolved from the fingerprints ("chrome"|"firefox"|"safari"|"go"|
	// "openssl"|"unknown"). Used by the L6 cross-check (SoT-02 §L6).
	JA4Engine string `json:"ja4Engine,omitempty"`
	H2Engine  string `json:"h2Engine,omitempty"`
	AkamaiH2  string `json:"akamaiH2,omitempty"`
	// SecFetchPresent / SecCHUAPresent record header presence for cross-checks.
	SecFetchPresent bool              `json:"secFetchPresent"`
	SecCHUAPresent  bool              `json:"secChUaPresent"`
	H2Settings      map[string]uint32 `json:"h2Settings,omitempty"`
	PseudoOrder     []string          `json:"pseudoHeaderOrder,omitempty"`
	HeaderOrder     []string          `json:"headerOrder,omitempty"`
	TLSVersion      string            `json:"tlsVersion,omitempty"`
	NegotiatedALPN  string            `json:"negotiatedAlpn,omitempty"`
	Signals         []Signal          `json:"signals"`
}

// SessionReport is the full merged record (SoT-00 §3.2).
type SessionReport struct {
	SessionID   string        `json:"sessionId"`
	Timestamp   time.Time     `json:"ts"`
	Client      ClientReport  `json:"client"`
	Network     NetworkReport `json:"network"`
	CrossChecks []CrossCheck  `json:"crosschecks"`
	Scoring     ScoreResult   `json:"scoring"`
	// Label is optional ground-truth for e2e evaluation ("human"|"bot:<tool>").
	Label string `json:"label,omitempty"`
}

// AllSignals returns client + network signals flattened, for scoring.
func (r *SessionReport) AllSignals() []Signal {
	out := make([]Signal, 0, len(r.Client.Signals)+len(r.Network.Signals))
	out = append(out, r.Client.Signals...)
	out = append(out, r.Network.Signals...)
	return out
}
