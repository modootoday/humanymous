package gate

import (
	"path"
	"strings"
	"time"
)

// config.go holds the minimal declarative route policy (SoT-24 §1, §2). A real
// deployment loads YAML/HCL with hot-reload + validation; this reference keeps a
// small in-code preset table so the enforcement path is exercised end-to-end.

// routePolicy is the resolved policy for a matched route.
type routePolicy struct {
	name       string
	inject     bool // inject the detection bundle into HTML (SoT-20)
	enforce    bool // false => monitor/shadow (score+log, act nothing) (SoT-24 §3)
	failClosed bool // Unknown verdict => challenge (sensitive routes) (SoT-21)
	syncScore  bool // synchronous pre-mutation re-score (high-assurance) (SoT-21 §2)
	// attestFloor is the ceiling-guard #1 lever (Blue-team ceiling-guard meeting): on a
	// high-value route the operator marks with this, a plain scoring-ALLOW no longer buys
	// the free fast-path — the session must ALSO present possession (a credential
	// trust-upgrade, handled by the pre-gates) OR a step-up proof (hmn_su, minted on an
	// LLM-resistant SoT-36 Pass solve). It does NOT detect a coherent spoof (undetectable
	// past the ceiling); it PRICES the ALLOW so the spoof's unlimited free high-value
	// passes become either possession-or-a-per-session human solve. CHALLENGE→Pass only,
	// never DENY, so a real unattested human just solves the same Pass an anonymous
	// visitor already solves (no categorical human block; no-lockout preserved).
	attestFloor bool
}

// Preset names.
var (
	presetStrict   = routePolicy{name: "strict", inject: true, enforce: true, failClosed: true, syncScore: true}
	presetBalanced = routePolicy{name: "balanced", inject: true, enforce: true, failClosed: false}
	presetMonitor  = routePolicy{name: "monitor", inject: true, enforce: false, failClosed: false}
	presetOff      = routePolicy{name: "off", inject: false, enforce: false, failClosed: false}
	// presetAttested is strict PLUS the attestation floor (ceiling-guard #1). Reserve it
	// for operator-marked high-value MUTATING routes (POST /checkout, /transfer, /password,
	// /api/keys) — never a public browse/catch-all route (config validation refuses that).
	presetAttested = routePolicy{name: "attested", inject: true, enforce: true, failClosed: true, syncScore: true, attestFloor: true}
)

// Config is the proxy's runtime configuration.
type Config struct {
	Upstream    string // upstream origin base URL (http[s]://host:port)
	NodeID      string
	ControlPath string        // reserved control-plane prefix (default /__hmn/)
	OriginKey   []byte        // origin-cloaking HMAC key (SoT-23 §1, HR-24)
	TokenKey    []byte        // verdict trust-token HMAC key (SoT-21 §3, HR-28)
	TokenEpochs *EpochManager // shared rotating token epoch (SoT-28 WS6); nil => own
	// Rate-limit -> auto-ban thresholds (SoT-27 §2). Zero uses defaults.
	RateWindow time.Duration
	RateSoft   int
	RateHard   int
	// ErasureHold is the cancellable grace window before a crypto-shred commits
	// (SoT-28 WS3). Zero uses the default.
	ErasureHold time.Duration
	// Routes maps a path glob prefix -> preset name; longest prefix wins.
	Routes map[string]string
	// GlobalMode overrides enforce->monitor when set to "monitor" (mandatory
	// first rollout stage, SoT-24 §3).
	GlobalMonitor bool
	// BanLedger, when non-nil, replaces the default in-memory ban store with a shared
	// (e.g. Redis) implementation so bans propagate across a fleet (PLAN-08 R1). nil =
	// the single-node in-memory BanStore, unchanged.
	BanLedger BanLedger
	// AgentKeys, when non-nil, enables Web Bot Auth signature verification at the edge
	// (PLAN-08 R3): a valid signature from an allowlisted key is a trust-upgrade, a
	// forgery is denied. nil = the feature is off.
	AgentKeys KeyDirectory
	// AnomalyShadow enables the PLAN-08 R5 shadow anomaly observer (log-only, never
	// affects the verdict). Off by default — the frozen detection path is untouched.
	AnomalyShadow bool
	// PATIssuers, when non-nil, enables Privacy Pass Private Access Token verification
	// at the edge (PLAN-08 R2): a valid token from a trusted issuer is a trust-upgrade.
	// nil = the feature is off.
	PATIssuers *PATVerifier
	// WebAuthnCreds, when non-nil, enables WebAuthn assertion verification at the edge
	// (PLAN-08 R2): a valid, fresh possession assertion from a registered credential is
	// a trust-upgrade. nil = the feature is off.
	WebAuthnCreds *WebAuthnRegistry
	// CredFanoutCap is the ceiling-guard #2 threshold: the number of distinct /24 subnets
	// a single WebAuthn credential may fast-path from within the tracking window before its
	// trust-upgrade reverts to Pass (a stolen/shared passkey fronting a proxy farm). Zero
	// uses DefaultCredFanoutCap. Only armed when WebAuthnCreds is set.
	CredFanoutCap int
	// MaxBodyBytes caps the request-body size forwarded to origin on the proxy path
	// (production ingress hardening). 0 = unlimited (reference default). A body over the
	// cap is rejected with 413 before it reaches origin. Gate's own control/admin JSON
	// endpoints keep their own tighter caps regardless of this value.
	MaxBodyBytes int64
	// HSTS adds a Strict-Transport-Security header to edge responses. Off by default —
	// enabling it in dev behind the self-signed cert would pin a bad cert in browsers.
	HSTS bool
	// VirtualUSBResultsDir enables a read-only operator view over bounded,
	// terminal virtual-USB ladder summaries. Empty disables the view. The
	// directory is never used by scoring, enforcement, or a mutation endpoint.
	VirtualUSBResultsDir string
}

// DefaultCredFanoutCap bounds a single WebAuthn credential's /24 fan-out before its
// fast-path reverts to Pass (ceiling-guard #2). Kept generous so a roaming human on
// one passkey across several networks in an hour is never inconvenienced — it bites
// only an obvious credential-farm fan-out. Degrades to CHALLENGE (Pass), never DENY.
const DefaultCredFanoutCap = 8

// resolve returns the policy for a request path (longest-prefix match; default
// balanced). GlobalMonitor downgrades enforce to monitor everywhere.
//
// The path is NORMALIZED (path.Clean + duplicate-slash collapse) and matched on SEGMENT
// boundaries before selecting a preset. A bare strings.HasPrefix on the raw path let
// `/./admin`, `//admin`, or `/admin/..;/` miss the `/admin` strict preset and fall through
// to the weaker balanced default — which passes an UNKNOWN verdict on a safe method — then
// be forwarded to an origin that collapses the path back to `/admin` and serves it with no
// enforcement (deep-review path-confusion bypass). Segment anchoring also stops a `/health`
// off-preset from over-scoping onto `/healthstatus-secret`. Only policy SELECTION is
// normalized here; the request forwarded upstream keeps its original path.
func (c Config) resolve(rawPath string) routePolicy {
	p := normalizeMatchPath(rawPath)
	best := presetBalanced
	bestLen := -1
	for prefix, preset := range c.Routes {
		if segmentMatch(p, prefix) && len(prefix) > bestLen {
			best = presetByName(preset)
			bestLen = len(prefix)
		}
	}
	if c.GlobalMonitor {
		best.enforce = false
	}
	return best
}

// normalizeMatchPath canonicalizes a request path for policy matching: it resolves `.`/`..`
// segments and collapses duplicate slashes via path.Clean, so equivalent encodings map to
// the single path an origin will actually serve. (net/http has already percent-decoded
// r.URL.Path, so `%2e` is already `.` here.)
func normalizeMatchPath(p string) string {
	if p == "" {
		return "/"
	}
	if !strings.HasPrefix(p, "/") {
		p = "/" + p
	}
	return path.Clean(p)
}

// segmentMatch reports whether prefix matches path on a path-segment boundary: an exact
// match, or path continues with a `/` after prefix. The root prefix matches everything.
func segmentMatch(p, prefix string) bool {
	if prefix == "" || prefix == "/" {
		return true
	}
	prefix = strings.TrimRight(prefix, "/")
	return p == prefix || strings.HasPrefix(p, prefix+"/")
}

// rate-limit defaults (SoT-27 §2): generous window/thresholds to avoid FP.
// Exported so a shared (Redis) ban ledger built outside NewServer uses the SAME
// thresholds and cannot drift from the in-memory path (PLAN-08 R1).
const (
	DefaultRateWindow = 10 * time.Second
	DefaultRateSoft   = 60
	DefaultRateHard   = 120
)

func rlWindow(c Config) time.Duration {
	if c.RateWindow > 0 {
		return c.RateWindow
	}
	return DefaultRateWindow
}
func rlSoft(c Config) int {
	if c.RateSoft > 0 {
		return c.RateSoft
	}
	return DefaultRateSoft
}
func rlHard(c Config) int {
	if c.RateHard > 0 {
		return c.RateHard
	}
	return DefaultRateHard
}
func erasureHold(c Config) time.Duration {
	if c.ErasureHold > 0 {
		return c.ErasureHold
	}
	return 5 * time.Minute
}

func presetByName(n string) routePolicy {
	switch n {
	case "strict":
		return presetStrict
	case "attested":
		return presetAttested
	case "monitor":
		return presetMonitor
	case "off":
		return presetOff
	default:
		return presetBalanced
	}
}
