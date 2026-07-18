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
		"10.20.30.40",  // private 10/8
		"192.168.1.9",  // private 192.168/16
		"172.30.0.5",   // private 172.16/12 (Docker)
		"127.0.0.1",    // loopback
		"169.254.1.1",  // link-local
		"0.0.0.0",      // unspecified
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
