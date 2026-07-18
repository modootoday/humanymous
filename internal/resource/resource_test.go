package resource

import (
	"testing"
	"time"
)

func TestClassify(t *testing.T) {
	cases := []struct {
		path, mime string
		size       int64
		dest       string
		want       Tier
	}{
		{"/app.js", "application/javascript", 1000, "script", TierEssential},
		{"/style.css", "text/css", 500, "style", TierEssential},
		{"/movie.mp4", "video/mp4", 5 << 20, "video", TierHeavy},
		{"/big.bin", "application/octet-stream", 3 << 20, "", TierHeavy},
		{"/photo.jpg", "image/jpeg", 500 << 10, "image", TierMedia},
		{"/icon.png", "image/png", 4 << 10, "image", TierLight},
		{"/frame.html", "text/html", 1000, "iframe", TierEmbed},
	}
	for _, c := range cases {
		if got := Classify(c.path, c.mime, c.size, c.dest); got != c.want {
			t.Errorf("Classify(%s,%s,%d,%s)=%v want %v", c.path, c.mime, c.size, c.dest, got, c.want)
		}
	}
}

func TestGateMatrix(t *testing.T) {
	cases := []struct {
		tier      Tier
		verdict   string
		rit, live bool
		want      Decision
	}{
		{TierEssential, VerdictDeny, false, false, Serve},    // essentials always served
		{TierHeavy, VerdictAllow, true, true, Serve},         // trusted human -> full video
		{TierHeavy, VerdictAllow, false, false, Challenge},   // allow but no RIT/liveness
		{TierHeavy, VerdictChallenge, true, true, Downgrade}, // preview only
		{TierHeavy, VerdictDeny, true, true, DenyServe},      // bot -> no video (save bandwidth)
		{TierEmbed, VerdictDeny, true, true, DenyServe},
		{TierLight, VerdictDeny, false, false, Serve}, // cheap, not worth blocking
	}
	for _, c := range cases {
		if got := Gate(c.tier, c.verdict, c.rit, c.live); got != c.want {
			t.Errorf("Gate(%v,%s,%v,%v)=%v want %v", c.tier, c.verdict, c.rit, c.live, got, c.want)
		}
	}
}

func TestMediaRangeStorm(t *testing.T) {
	m := NewMediaTracker()
	base := time.Unix(1000, 0)
	var stormed bool
	for i := 0; i < 10; i++ {
		sigs := m.Observe(MediaEvent{SessionID: "s1", IsRange: true, HasRIT: true, At: base.Add(time.Duration(i) * 100 * time.Millisecond)})
		for _, s := range sigs {
			if s.ID == "l5.media.range_storm" {
				stormed = true
			}
		}
	}
	if !stormed {
		t.Fatal("expected range_storm after rapid range requests")
	}
}

func TestMediaNoStormWhenSpread(t *testing.T) {
	m := NewMediaTracker()
	base := time.Unix(1000, 0)
	for i := 0; i < 10; i++ {
		sigs := m.Observe(MediaEvent{SessionID: "s2", IsRange: true, HasRIT: true, At: base.Add(time.Duration(i) * time.Second)})
		for _, s := range sigs {
			if s.ID == "l5.media.range_storm" {
				t.Fatal("spread-out ranges should not storm")
			}
		}
	}
}

func TestSignedURL_ExpiryAndTamper(t *testing.T) {
	mk := []byte("key")
	sig := Sign(mk, "sess", "/res/movie.mp4", 2000)
	if !Verify(mk, "sess", "/res/movie.mp4", 2000, 1500, sig) {
		t.Fatal("valid signed url rejected")
	}
	if Verify(mk, "sess", "/res/movie.mp4", 2000, 2500, sig) {
		t.Fatal("expired url accepted")
	}
	if Verify(mk, "other", "/res/movie.mp4", 2000, 1500, sig) {
		t.Fatal("wrong session accepted")
	}
	if Verify(mk, "sess", "/res/movie.mp4", 2000, 1500, "tampered") {
		t.Fatal("tampered sig accepted")
	}
}

func TestEmbedTokenStripped(t *testing.T) {
	sigs := Inspect(EmbedContext{SecFetchDest: "iframe", Origin: "https://evil.com",
		AllowedHosts: []string{"myapp.local"}, RITPresent: false})
	var stripped, badOrigin bool
	for _, s := range sigs {
		if s.ID == "l5.embed.token_stripped" {
			stripped = true
		}
		if s.ID == "l5.embed.origin_disallowed" {
			badOrigin = true
		}
	}
	if !stripped {
		t.Error("expected token_stripped")
	}
	if !badOrigin {
		t.Error("expected origin_disallowed (non-empty allow-list needed)")
	}
}
