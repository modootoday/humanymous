package network

import "testing"

// buildIDs returns the signal ids emitted for an Observation.
func buildIDs(obs Observation) map[string]bool {
	nr := Build(obs)
	ids := make(map[string]bool, len(nr.Signals))
	for _, s := range nr.Signals {
		ids[s.ID] = true
	}
	return ids
}

func TestForwardedPrivateSpoofSignal(t *testing.T) {
	fires := []string{
		"10.20.30.40", // private 10/8
		"192.168.1.9", // private 192.168/16
		"172.30.0.5",  // private 172.16/12 (Docker)
		"127.0.0.1",   // loopback
		"169.254.1.1", // link-local
		"0.0.0.0",     // unspecified
	}
	for _, ip := range fires {
		if !buildIDs(Observation{ClientForwardedIP: ip})["l5.header.forwarded_private"] {
			t.Errorf("forwarded IP %q should raise l5.header.forwarded_private", ip)
		}
	}

	quiet := []string{
		"",            // no forwarding header
		"203.0.113.9", // public — a real proxy forwards a public client IP
		"8.8.8.8",     // public
		"garbage",     // unparseable
	}
	for _, ip := range quiet {
		if buildIDs(Observation{ClientForwardedIP: ip})["l5.header.forwarded_private"] {
			t.Errorf("forwarded IP %q must NOT raise l5.header.forwarded_private", ip)
		}
	}
}

func TestDatacenterSignalGatedByObservation(t *testing.T) {
	if !buildIDs(Observation{IsDatacenterIP: true})["l5.ip.datacenter_asn"] {
		t.Error("IsDatacenterIP=true should raise l5.ip.datacenter_asn")
	}
	if buildIDs(Observation{IsDatacenterIP: false})["l5.ip.datacenter_asn"] {
		t.Error("IsDatacenterIP=false must not raise l5.ip.datacenter_asn")
	}
}

func TestProxyHopSquidVia(t *testing.T) {
	ids := buildIDs(Observation{Header: HeaderInfo{
		Via: "1.1 proxy.example (squid/5.7)",
	}})
	if !ids["l5.header.proxy_hop"] {
		t.Fatal("Squid Via must raise l5.header.proxy_hop")
	}
}

func TestProxyHopProxyConnection(t *testing.T) {
	ids := buildIDs(Observation{Header: HeaderInfo{ProxyConnection: "keep-alive"}})
	if !ids["l5.header.proxy_hop"] {
		t.Fatal("Proxy-Connection must raise l5.header.proxy_hop")
	}
}

func TestXFFMultiHop(t *testing.T) {
	ids := buildIDs(Observation{Header: HeaderInfo{
		XForwardedFor: "203.0.113.1, 198.51.100.2",
	}})
	if !ids["l5.header.xff_multi_hop"] {
		t.Fatal("multi-hop XFF must raise l5.header.xff_multi_hop")
	}
	ids1 := buildIDs(Observation{Header: HeaderInfo{XForwardedFor: "203.0.113.1"}})
	if ids1["l5.header.xff_multi_hop"] {
		t.Fatal("single-hop XFF must NOT raise xff_multi_hop")
	}
}

func TestProxyVPNIPSignal(t *testing.T) {
	if !buildIDs(Observation{IsProxy: true})["l5.ip.proxy_vpn_tor"] {
		t.Fatal("IsProxy=true should raise l5.ip.proxy_vpn_tor")
	}
	if buildIDs(Observation{IsProxy: false})["l5.ip.proxy_vpn_tor"] {
		t.Fatal("IsProxy=false must not raise proxy_vpn_tor")
	}
}

func TestClientIPSpoofAndAnonChain(t *testing.T) {
	if !buildIDs(Observation{Header: HeaderInfo{CFConnectingIP: "8.8.8.8"}})["l5.header.client_ip_spoof"] {
		t.Fatal("CF-Connecting-IP must raise client_ip_spoof")
	}
	ids := buildIDs(Observation{Header: HeaderInfo{
		XForwardedFor: "1.1.1.1, 2.2.2.2, 3.3.3.3, 4.4.4.4",
	}})
	if !ids["l5.proxy.anon_chain"] {
		t.Fatal("≥4-hop XFF must raise anon_chain")
	}
	if !ids["l5.proxy.tor_circuit"] {
		t.Fatal("≥4-hop also implies tor_circuit (≥3)")
	}
}

func TestTorExitAndCircuitSignals(t *testing.T) {
	if !buildIDs(Observation{IsTorExit: true})["l5.ip.tor_exit"] {
		t.Fatal("IsTorExit=true should raise l5.ip.tor_exit")
	}
	ids := buildIDs(Observation{Header: HeaderInfo{
		XForwardedFor: "185.220.101.1, 185.220.102.2, 203.0.113.50",
	}})
	if !ids["l5.proxy.tor_circuit"] {
		t.Fatal("≥3-hop XFF must raise l5.proxy.tor_circuit")
	}
	if !ids["l5.header.xff_multi_hop"] {
		t.Fatal("≥3-hop XFF must also raise xff_multi_hop")
	}
	ids2 := buildIDs(Observation{Header: HeaderInfo{XForwardedFor: "203.0.113.1, 198.51.100.2"}})
	if ids2["l5.proxy.tor_circuit"] {
		t.Fatal("2-hop XFF must NOT raise tor_circuit (needs ≥3)")
	}
}
