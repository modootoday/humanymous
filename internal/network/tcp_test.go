package network

import "testing"

func TestTCPNotObservedWhenEmpty(t *testing.T) {
	sigs := TCPSignals(TCPObservation{}, "Mozilla/5.0 (Windows NT 10.0) Chrome/126")
	ids := map[string]bool{}
	for _, s := range sigs {
		ids[s.ID] = true
	}
	if !ids["l5.tcp.not_observed"] {
		t.Fatal("empty TCP observation must emit l5.tcp.not_observed")
	}
}

func TestTCPTTLOSMismatchAuditOnly(t *testing.T) {
	sigs := TCPSignals(TCPObservation{
		Observed: true, TTL: 64, OSFamily: "linux", MSS: 1460, Window: 65535,
	}, "Mozilla/5.0 (Windows NT 10.0) Chrome/126")
	var ttlOS bool
	for _, s := range sigs {
		if s.ID == "l5.tcp.ttl_os" {
			ttlOS = true
		}
		if IsScoreExempt(s.ID) != true && s.ID != "" {
			// all TCP signals must be score-exempt
		}
	}
	if !ttlOS {
		t.Fatal("linux TTL + Windows UA must raise l5.tcp.ttl_os")
	}
	for _, s := range sigs {
		if !IsScoreExempt(s.ID) {
			t.Fatalf("%s must be score-exempt", s.ID)
		}
	}
}

func TestAuditEventMapping(t *testing.T) {
	if AuditEventFor("l5.header.proxy_hop") != "net.proxy.hop" {
		t.Fatal("proxy_hop audit mapping")
	}
	if AuditEventFor("l5.tcp.ttl_os") != "net.tcp.ttl_os_mismatch" {
		t.Fatal("ttl_os audit mapping")
	}
	if AuditEventFor("l5.rit.absent") != "" {
		t.Fatal("RIT is not a network residual audit event")
	}
}
