package trafficguard

import "github.com/modootoday/humanymous/internal/signals"

// consistency.go inspects a session's traffic records for rotation/evasion
// across requests and emits l5.traffic.* signals (SoT-12 §2). A real browser
// keeps one TLS stack, one UA, and a stable network path for a session; rotation
// is strong evidence of a bypass attempt.

// Check evaluates the intra-session consistency of a session's records.
func Check(recs []TrafficRecord) []signals.Signal {
	if len(recs) < 2 {
		return nil // need at least two requests to compare
	}
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		out = append(out, signals.New(id, val, v, 1.0, signals.SourceServer, notes))
	}

	var firstEngine, firstJA4, firstUA, firstSubnet string
	var httpProtos = map[string]int{}
	engineSet, ja4Set, uaSet, subnetSet := set{}, set{}, set{}, set{}
	uaToOrder := map[string]set{}
	extOrderSet := set{}
	chromeTLSConns := 0

	for _, r := range recs {
		if r.JA4Engine != "" {
			engineSet.add(r.JA4Engine)
			if firstEngine == "" {
				firstEngine = r.JA4Engine
			}
		}
		if r.JA4 != "" {
			ja4Set.add(ja4Stable(r.JA4))
			if firstJA4 == "" {
				firstJA4 = r.JA4
			}
		}
		if r.UAHash != "" {
			uaSet.add(r.UAHash)
			if firstUA == "" {
				firstUA = r.UAHash
			}
			if r.HeaderOrder != "" {
				s := uaToOrder[r.UAHash]
				if s == nil {
					s = set{}
				}
				s.add(r.HeaderOrder)
				uaToOrder[r.UAHash] = s
			}
		}
		if r.RemoteAddr != "" {
			sn := Subnet24(r.RemoteAddr)
			subnetSet.add(sn)
			if firstSubnet == "" {
				firstSubnet = sn
			}
		}
		if r.Proto != "" {
			httpProtos[r.Proto]++
		}
		// Track TLS extension-order across chrome-engine connections for the
		// permutation check (SoT-14 §B1).
		if r.JA4Engine == "chrome" && r.ExtOrder != "" {
			chromeTLSConns++
			extOrderSet.add(r.ExtOrder)
		}
	}

	if engineSet.len() > 1 {
		add("l5.traffic.engine_rotation", engineSet.keys(), signals.VerdictBot, "TLS engine family rotated within session")
	}
	// A benign TLS 1.3 session resumption changes the ClientHello (it adds pre_shared_key /
	// early_data), which shifts the JA4_a extension count and therefore the stable JA4 — so a
	// legitimate browser reusing a session ticket presents the full + resumed pair (2 distinct
	// stable JA4s) with no automation at all. Flag rotation only at 3+ distinct stable
	// fingerprints, which resumption cannot explain, so HR-14/HR-15 no longer hard-DENY a real
	// resuming browser (deep-review no-lockout). Genuine TLS-stack rotation is also caught by
	// engine_rotation (a different engine family) above, which is unaffected by this tolerance.
	if ja4Set.len() > 2 {
		add("l5.traffic.ja4_rotation", ja4Set.len(), signals.VerdictBot, "JA4 (stable part) rotated within session")
	}
	if uaSet.len() > 1 {
		add("l5.traffic.ua_rotation", uaSet.len(), signals.VerdictBot, "User-Agent rotated within session")
	}
	if subnetSet.len() > 1 {
		add("l5.traffic.ip_hop", subnetSet.keys(), signals.VerdictSuspicious, "remote subnet changed within session")
	}
	// TLS permutation (SoT-14 §B1): real Chrome 110+ permutes its extension
	// order every connection, so a single order across many chrome-engine
	// connections would indicate a STATIC parrot. DISABLED by default because
	// automation-launched Chromium/Edge (e.g. Playwright-driven) does not
	// reliably permute in every environment, which false-positives real
	// browsers. The header-spoofing HTTP parrot is instead caught by
	// x.browser_no_js (no JS-execution evidence). Re-enable only where the real
	// browser population is known to permute. (var kept referenced below.)
	_ = extOrderSet
	_ = chromeTLSConns
	if httpProtos["1.1"] > 0 && httpProtos["2"] > 0 {
		add("l5.traffic.proto_downgrade", httpProtos, signals.VerdictSuspicious, "mixed h2/h1 within session")
	}
	// header_order_shift DISABLED: Go's net/http does not preserve on-wire header
	// order (map-backed), and a real browser legitimately sends different header
	// SETS for navigate vs cors vs no-cors requests, so any order/set hash varies
	// benignly across a normal session -> false positive. Real detection needs
	// raw wire capture (plan/02). Kept for reference only.
	_ = uaToOrder
	return out
}

// ja4Stable returns the permutation-invariant portion of a JA4 string: the
// JA4_b (sorted cipher hash) is stable across Chrome's extension permutation,
// so we compare on a+b, ignoring the c segment which can vary by SNI/ALPN.
func ja4Stable(ja4 string) string {
	// ja4 = a_b_c ; keep a_b.
	first := -1
	for i := 0; i < len(ja4); i++ {
		if ja4[i] == '_' {
			if first == -1 {
				first = i
			} else {
				return ja4[:i]
			}
		}
	}
	return ja4
}

// set is a tiny string set helper.
type set map[string]struct{}

func (s set) add(k string) { s[k] = struct{}{} }
func (s set) len() int     { return len(s) }
func (s set) keys() []string {
	out := make([]string, 0, len(s))
	for k := range s {
		out = append(out, k)
	}
	return out
}
