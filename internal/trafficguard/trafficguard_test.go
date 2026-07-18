package trafficguard

import (
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

func rec(sid, addr, engine, ja4, ua, proto string) TrafficRecord {
	return TrafficRecord{
		TS: time.Unix(1, 0), SessionID: sid, RemoteAddr: addr,
		JA4Engine: engine, JA4: ja4, UAHash: HashUA(ua), Proto: proto, Method: "GET",
	}
}

func TestConsistency_EngineRotationDetected(t *testing.T) {
	recs := []TrafficRecord{
		rec("s1", "1.2.3.4:100", "chrome", "t13d1516h2_aaa_bbb", "Chrome/126", "2"),
		rec("s1", "1.2.3.4:101", "go", "t12i0000_ccc_ddd", "Chrome/126", "2"),
	}
	sigs := Check(recs)
	if !contains(sigs, "l5.traffic.engine_rotation") {
		t.Fatalf("expected engine_rotation, got %v", ids(sigs))
	}
}

func TestConsistency_UARotationDetected(t *testing.T) {
	recs := []TrafficRecord{
		rec("s1", "1.2.3.4:100", "chrome", "t13d1516h2_aaa_bbb", "Chrome/126", "2"),
		rec("s1", "1.2.3.4:101", "chrome", "t13d1516h2_aaa_bbb", "Firefox/130", "2"),
	}
	sigs := Check(recs)
	if !contains(sigs, "l5.traffic.ua_rotation") {
		t.Fatalf("expected ua_rotation, got %v", ids(sigs))
	}
}

func TestConsistency_StableSessionClean(t *testing.T) {
	recs := []TrafficRecord{
		rec("s1", "1.2.3.4:100", "chrome", "t13d1516h2_aaa_bbb", "Chrome/126", "2"),
		rec("s1", "1.2.3.4:101", "chrome", "t13d1516h2_aaa_bbb", "Chrome/126", "2"),
		rec("s1", "1.2.3.4:102", "chrome", "t13d1516h2_aaa_bbb", "Chrome/126", "2"),
	}
	if sigs := Check(recs); len(sigs) != 0 {
		t.Fatalf("stable session should be clean, got %v", ids(sigs))
	}
}

func TestConsistency_JA4PermutationTolerated(t *testing.T) {
	// Same a_b (stable) but different c segment (SNI/ALPN) must NOT rotate.
	recs := []TrafficRecord{
		rec("s1", "1.2.3.4:100", "chrome", "t13d1516h2_aaa_bbb", "Chrome/126", "2"),
		rec("s1", "1.2.3.4:101", "chrome", "t13d1516h2_aaa_ZZZ", "Chrome/126", "2"),
	}
	if contains(Check(recs), "l5.traffic.ja4_rotation") {
		t.Fatal("JA4 c-segment change should be tolerated (permutation)")
	}
}

func TestLog_BackfillTLS(t *testing.T) {
	l := NewLog(time.Hour)
	l.RecordTLS(TrafficRecord{RemoteAddr: "1.2.3.4:100", JA4: "t13d1516h2_a_b", JA4Engine: "chrome", TS: time.Unix(1, 0)})
	l.RecordHTTP(TrafficRecord{SessionID: "s1", RemoteAddr: "1.2.3.4:100", Method: "GET", Path: "/", TS: time.Unix(2, 0)})
	recs := l.Session("s1")
	if len(recs) != 1 || recs[0].JA4Engine != "chrome" {
		t.Fatalf("TLS not backfilled into HTTP record: %+v", recs)
	}
}

// TestConsistency_HeaderOrderNilMap guards the nil-map panic that surfaced only
// when records carried a HeaderOrder (the browser sends one; unit fixtures did not).
func TestConsistency_HeaderOrderTracked(t *testing.T) {
	mk := func(order string) TrafficRecord {
		r := rec("s1", "1.2.3.4:100", "chrome", "t13d1516h2_a_b", "Chrome/126", "2")
		r.HeaderOrder = order
		return r
	}
	// header_order_shift is disabled (Go loses wire order; header sets vary
	// benignly). Many distinct order hashes must NOT flag, and must not panic.
	if contains(Check([]TrafficRecord{mk("h1"), mk("h2"), mk("h3"), mk("h4"), mk("h5")}), "l5.traffic.header_order_shift") {
		t.Fatal("header_order_shift should be disabled (FP-prone)")
	}
	// stable session with header orders -> no panic, clean.
	if len(Check([]TrafficRecord{mk("nav"), mk("cors"), mk("nav")})) != 0 {
		t.Fatal("stable session should be clean")
	}
}

func TestSubnet24(t *testing.T) {
	if Subnet24("192.168.5.9:443") != "192.168.5" {
		t.Errorf("subnet mismatch: %s", Subnet24("192.168.5.9:443"))
	}
}

func contains(sigs []signals.Signal, id string) bool {
	for _, s := range sigs {
		if s.ID == id {
			return true
		}
	}
	return false
}
func ids(sigs []signals.Signal) []string {
	var out []string
	for _, s := range sigs {
		out = append(out, s.ID)
	}
	return out
}
