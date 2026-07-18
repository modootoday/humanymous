package sentinel

import (
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
}

// Preset names.
var (
	presetStrict   = routePolicy{name: "strict", inject: true, enforce: true, failClosed: true, syncScore: true}
	presetBalanced = routePolicy{name: "balanced", inject: true, enforce: true, failClosed: false}
	presetMonitor  = routePolicy{name: "monitor", inject: true, enforce: false, failClosed: false}
	presetOff      = routePolicy{name: "off", inject: false, enforce: false, failClosed: false}
)

// Config is the proxy's runtime configuration.
type Config struct {
	Upstream    string // upstream origin base URL (http[s]://host:port)
	NodeID      string
	ControlPath string // reserved control-plane prefix (default /__hmn/)
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
}

// resolve returns the policy for a request path (longest-prefix match; default
// balanced). GlobalMonitor downgrades enforce to monitor everywhere.
func (c Config) resolve(path string) routePolicy {
	best := presetBalanced
	bestLen := -1
	for prefix, preset := range c.Routes {
		if strings.HasPrefix(path, prefix) && len(prefix) > bestLen {
			best = presetByName(preset)
			bestLen = len(prefix)
		}
	}
	if c.GlobalMonitor {
		best.enforce = false
	}
	return best
}

// rate-limit defaults (SoT-27 §2): generous window/thresholds to avoid FP.
func rlWindow(c Config) time.Duration {
	if c.RateWindow > 0 {
		return c.RateWindow
	}
	return 10 * time.Second
}
func rlSoft(c Config) int {
	if c.RateSoft > 0 {
		return c.RateSoft
	}
	return 60
}
func rlHard(c Config) int {
	if c.RateHard > 0 {
		return c.RateHard
	}
	return 120
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
	case "monitor":
		return presetMonitor
	case "off":
		return presetOff
	default:
		return presetBalanced
	}
}
