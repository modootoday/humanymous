package network

import (
	"strings"

	"github.com/modootoday/humanymous/internal/signals"
)

// tcp.go is the TCP/L4 observation plane (OSI L3–L4 residual). Captures are
// optional: full ClientHello is already handled elsewhere; TCP SYN options
// (TTL, MSS, window scale, SACK) typically require PROXY-protocol TLVs, a
// kernel/eBPF sniffer, or a raw socket — not available from pure net/http.
//
// Design (product): these residuals NEVER contribute to the risk score. They
// are observational signals for the Audit plane so operators can detect and
// manually ban (ip:/fp:/cidr:). Auto hard-rule promotion for pure TCP residual
// is intentionally absent — admin ownership, not silent score pressure.

// TCPObservation is one connection's L4 residual when the capture plane is live.
type TCPObservation struct {
	// Observed is false when no SYN/PROXY TLV/eBPF capture is wired (common on
	// Gate and Core behind TLS terminators). Emit l5.tcp.not_observed then.
	Observed bool
	TTL      uint8  // IP TTL at capture (often 64 Linux, 128 Windows)
	MSS      uint16 // TCP MSS option, 0 if unknown
	Window   uint16 // initial window, 0 if unknown
	WScale   int8   // window scale option (-1 = absent)
	SACK     bool   // SACK permitted
	// OSFamily is a coarse guess from TTL/window heuristics: linux|windows|macos|unknown.
	OSFamily string
}

// GuessOSFromTTL maps common initial TTLs to an OS family (best-effort).
func GuessOSFromTTL(ttl uint8) string {
	switch {
	case ttl == 0:
		return "unknown"
	case ttl <= 64:
		return "linux" // also BSD/macOS often land here after hops
	case ttl <= 128:
		return "windows"
	default:
		return "unknown"
	}
}

// TCPSignals emits observational L4 residuals. All have weight 0 (score-exempt).
func TCPSignals(tcp TCPObservation, ua string) []signals.Signal {
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		out = append(out, signals.New(id, val, v, 1.0, signals.SourceServer, notes))
	}
	if !tcp.Observed {
		add("l5.tcp.not_observed", true, signals.VerdictUnknown,
			"no TCP SYN/PROXY-TLV capture — L4 plane inactive (wire eBPF/PROXY TLV for full residual)")
		return out
	}
	if tcp.OSFamily == "" && tcp.TTL > 0 {
		tcp.OSFamily = GuessOSFromTTL(tcp.TTL)
	}
	add("l5.tcp.ttl", tcp.TTL, signals.VerdictUnknown, "observed IP TTL")
	if tcp.MSS > 0 {
		add("l5.tcp.mss", tcp.MSS, signals.VerdictUnknown, "TCP MSS option")
	}
	if tcp.Window > 0 {
		add("l5.tcp.window", tcp.Window, signals.VerdictUnknown, "TCP initial window")
	}
	// TTL/OS vs claimed UA OS — observational residual only (admin audit), not score.
	claimed := uaOSFamily(ua)
	if tcp.OSFamily != "" && tcp.OSFamily != "unknown" && claimed != "" && claimed != "unknown" &&
		tcp.OSFamily != claimed {
		add("l5.tcp.ttl_os", map[string]string{"tcp": tcp.OSFamily, "ua": claimed},
			signals.VerdictSuspicious, "TCP TTL/OS family vs User-Agent OS mismatch")
	}
	return out
}

func uaOSFamily(ua string) string {
	u := strings.ToLower(ua)
	switch {
	case strings.Contains(u, "windows"):
		return "windows"
	case strings.Contains(u, "android"):
		return "linux"
	case strings.Contains(u, "iphone"), strings.Contains(u, "ipad"), strings.Contains(u, "mac os"):
		return "macos"
	case strings.Contains(u, "linux"), strings.Contains(u, "cros"):
		return "linux"
	default:
		return "unknown"
	}
}
