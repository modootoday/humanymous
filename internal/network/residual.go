package network

import "github.com/modootoday/humanymous/internal/signals"

// residual.go classifies NETWORK-PLANE residuals that are score-exempt and
// owned by the Audit / admin ban path:
//
//   - Proxy/VPN/Tor hop residuals (Via, multi-hop XFF, CDN IP spoof, WebRTC leak…)
//   - TCP/L4 residuals (TTL/MSS/window/OS)
//   - IP-intel residual markers (datacenter, proxy ASN, Tor exit list)
//   - Cross-session network correlation (proxy rotation, mid-session fp churn)
//
// These still appear on SessionReport.Network.Signals for console/TopSignals
// visibility, but ComputeScore uses weight 0 so they never move the risk dial.
// Operators act via Audit (detect) and Ban (block) — not silent score pressure.

// ScoreExempt residual ids — never contribute to Combine risk.
var scoreExemptIDs = map[string]struct{}{
	// HTTP hop / proxy / VPN / Tor
	"l5.header.proxy_hop":       {},
	"l5.header.client_ip_spoof": {},
	"l5.header.xff_multi_hop":   {},
	"l5.header.forwarded_private": {},
	"l5.proxy.vpn_webrtc_leak":  {},
	"l5.proxy.tor_circuit":      {},
	"l5.proxy.anon_chain":       {},
	"l5.ip.datacenter_asn":      {},
	"l5.ip.proxy_vpn_tor":       {},
	"l5.ip.tor_exit":            {},
	// TCP/L4 plane
	"l5.tcp.not_observed": {},
	"l5.tcp.ttl":          {},
	"l5.tcp.mss":          {},
	"l5.tcp.window":       {},
	"l5.tcp.ttl_os":       {},
	// Network correlation residual (admin-owned; policy may still hard-DENY)
	"l5.correlation.proxy_rotation":  {},
	"l5.correlation.fp_churn_proxy":  {},
	"l5.correlation.shared_fingerprint": {},
	"l5.traffic.ip_hop":              {},
}

// IsScoreExempt reports whether a signal is network-plane residual (audit/admin).
func IsScoreExempt(id string) bool {
	_, ok := scoreExemptIDs[id]
	return ok
}

// AuditEventFor maps a residual signal id to an audit event_type, or "".
func AuditEventFor(id string) string {
	switch id {
	case "l5.header.proxy_hop":
		return "net.proxy.hop"
	case "l5.header.client_ip_spoof":
		return "net.proxy.client_ip_spoof"
	case "l5.header.xff_multi_hop":
		return "net.proxy.xff_multi_hop"
	case "l5.header.forwarded_private":
		return "net.proxy.forwarded_private"
	case "l5.proxy.vpn_webrtc_leak":
		return "net.vpn.webrtc_leak"
	case "l5.proxy.tor_circuit":
		return "net.tor.circuit"
	case "l5.proxy.anon_chain":
		return "net.proxy.anon_chain"
	case "l5.ip.datacenter_asn":
		return "net.ip.datacenter"
	case "l5.ip.proxy_vpn_tor":
		return "net.ip.proxy_vpn"
	case "l5.ip.tor_exit":
		return "net.tor.exit"
	case "l5.tcp.not_observed":
		return "net.tcp.not_observed"
	case "l5.tcp.ttl", "l5.tcp.mss", "l5.tcp.window":
		return "net.tcp.fingerprint"
	case "l5.tcp.ttl_os":
		return "net.tcp.ttl_os_mismatch"
	case "l5.correlation.proxy_rotation":
		return "net.correlation.proxy_rotation"
	case "l5.correlation.fp_churn_proxy":
		return "net.correlation.fp_churn"
	case "l5.correlation.shared_fingerprint":
		return "net.correlation.shared_fingerprint"
	case "l5.traffic.ip_hop":
		return "net.traffic.ip_hop"
	default:
		return ""
	}
}

// ResidualSignals filters a signal list to score-exempt network residuals.
func ResidualSignals(sigs []signals.Signal) []signals.Signal {
	var out []signals.Signal
	for _, s := range sigs {
		if IsScoreExempt(s.ID) {
			out = append(out, s)
		}
	}
	return out
}

// HasActionableResidual reports whether any residual is BOT or SUSPICIOUS
// (admin-relevant; used for optional network policy / console badges).
func HasActionableResidual(sigs []signals.Signal) bool {
	for _, s := range ResidualSignals(sigs) {
		if s.Verdict == signals.VerdictBot || s.Verdict == signals.VerdictSuspicious {
			return true
		}
	}
	return false
}
