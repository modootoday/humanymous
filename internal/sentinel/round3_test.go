package sentinel

import (
	"net/http/httptest"
	"testing"
	"time"
)

// HR-23: request-framing smuggling primitives are detected.
func TestSmuggleScan(t *testing.T) {
	cases := []struct {
		name  string
		setup func(*httptest.ResponseRecorder)
		hdr   map[string][]string
		want  smuggleReason
	}{
		{"clean", nil, map[string][]string{"Content-Length": {"5"}}, smuggleNone},
		{"cl+te", nil, map[string][]string{"Content-Length": {"5"}, "Transfer-Encoding": {"chunked"}}, smuggleTECL},
		{"dup-cl", nil, map[string][]string{"Content-Length": {"5", "6"}}, smuggleDupCL},
		{"te-not-chunked", nil, map[string][]string{"Transfer-Encoding": {"chunked, chunked"}}, smuggleBadTE},
	}
	for _, c := range cases {
		r := httptest.NewRequest("POST", "http://p/", nil)
		r.Header = map[string][]string{}
		for k, v := range c.hdr {
			r.Header[k] = v
		}
		if got := smuggleScan(r); got != c.want {
			t.Errorf("%s: got %q want %q", c.name, got, c.want)
		}
	}
}

// HR-30: a fingerprint spinning up many sessions is flagged as a sweep.
func TestSweepDetector(t *testing.T) {
	d := NewSweepDetector(time.Minute, 3)
	now := time.Unix(1000, 0)
	flagged := false
	for i := 0; i < 6; i++ {
		if d.Observe("bindX", "sid"+string(rune('a'+i)), now) {
			flagged = true
		}
	}
	if !flagged {
		t.Fatal("6 sessions from one binding should trip the sweep detector")
	}
	// A different binding with few sessions is not flagged.
	if d.Observe("bindY", "s1", now) {
		t.Fatal("single session must not flag")
	}
}

// HR-30: sessions from one binding spread beyond the window do not accumulate.
func TestSweepWindowExpiry(t *testing.T) {
	d := NewSweepDetector(10*time.Second, 3)
	base := time.Unix(1000, 0)
	for i := 0; i < 3; i++ {
		d.Observe("b", "s"+string(rune('a'+i)), base.Add(time.Duration(i)*11*time.Second))
	}
	// each was >window apart, so the window reset each time -> never flags.
	if d.Observe("b", "sZ", base.Add(100*time.Second)) {
		t.Fatal("spread-out sessions must not flag")
	}
}
