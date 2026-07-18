package correlation

import (
	"testing"
	"time"
)

func TestProxyRotationDetected(t *testing.T) {
	r := New(time.Hour)
	now := time.Unix(1, 0)
	key := "fpABC|t13d1516h2_aaa"
	var fired bool
	// same fingerprint from 3 distinct subnets (rotating residential proxy).
	for i, subnet := range []string{"1.2.3", "9.8.7", "50.60.70"} {
		sigs := r.Observe(key, subnet, "sess-"+string(rune('a'+i)), now)
		for _, s := range sigs {
			if s.ID == "l5.correlation.proxy_rotation" {
				fired = true
			}
		}
	}
	if !fired {
		t.Fatalf("expected proxy_rotation across 3 subnets")
	}
}

func TestSharedFingerprintDetected(t *testing.T) {
	r := New(time.Hour)
	now := time.Unix(1, 0)
	key := "fpXYZ|ja4"
	var fired bool
	// 5 sessions, same subnet, same fingerprint (coordinated botnet).
	for i := 0; i < 5; i++ {
		sigs := r.Observe(key, "1.2.3", "s"+string(rune('0'+i)), now)
		for _, s := range sigs {
			if s.ID == "l5.correlation.shared_fingerprint" {
				fired = true
			}
		}
	}
	if !fired {
		t.Fatalf("expected shared_fingerprint across 5 sessions")
	}
}

func TestSingleSessionClean(t *testing.T) {
	r := New(time.Hour)
	if sigs := r.Observe("fp|ja4", "1.2.3", "sess-1", time.Unix(1, 0)); len(sigs) != 0 {
		t.Fatalf("single session should be clean, got %v", sigs)
	}
}

func TestEmptyKeyIgnored(t *testing.T) {
	r := New(time.Hour)
	if sigs := r.Observe("", "1.2.3", "s", time.Unix(1, 0)); sigs != nil {
		t.Fatal("empty key must be ignored")
	}
}
