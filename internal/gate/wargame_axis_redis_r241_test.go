package gate

import (
	"strings"
	"testing"
)

// Axis C (r241+): distributed ledger authenticity — Redis-backed verdict/ban values
// must resist forge / relocate / inject (fleet coordinator compromise model).

func TestWargameR241_RedisSealOpenRoundTrip(t *testing.T) {
	key := []byte("wargame-r241-fleet-secret!!!!")
	ctx := "hmn:verdict:sid-r241"
	sealed := sealValue(key, ctx, `{"v":"ALLOW","s":1}`)
	got, ok := openValue(key, ctx, sealed)
	if !ok || !strings.Contains(got, "ALLOW") {
		t.Fatalf("got %q ok=%v", got, ok)
	}
}

func TestWargameR242_RedisRelocationRejected(t *testing.T) {
	// Web/architecture: shared store without key binding enables cross-session transplant.
	key := []byte("wargame-r242-fleet-secret!!!!")
	sealed := sealValue(key, "hmn:verdict:sid-A", `{"v":"ALLOW"}`)
	if _, ok := openValue(key, "hmn:verdict:sid-B", sealed); ok {
		t.Fatal("relocated sealed value must fail openValue")
	}
}

func TestWargameR243_RedisUnsignedInjectionRejected(t *testing.T) {
	key := []byte("wargame-r243-fleet-secret!!!!")
	if _, ok := openValue(key, "hmn:verdict:sid", `{"v":"ALLOW"}`); ok {
		t.Fatal("unsigned JSON must not open in signed mode")
	}
}

func TestWargameR244_RedisTamperedPayloadRejected(t *testing.T) {
	key := []byte("wargame-r244-fleet-secret!!!!")
	ctx := "hmn:ban:ip:203.0.113.244"
	sealed := sealValue(key, ctx, `{"until":0}`)
	// splice deny payload with stolen tag suffix if present
	if len(sealed) < 32 {
		t.Fatal("sealed too short")
	}
	forged := `{"until":0,"evil":true}` + sealed[len(sealed)/2:]
	if _, ok := openValue(key, ctx, forged); ok {
		t.Fatal("tampered payload must fail")
	}
}

func TestWargameR245_RedisNilKeyPassthroughCompat(t *testing.T) {
	// Documented compatibility mode — not a security claim when key is nil.
	got, ok := openValue(nil, "ctx", `raw-legacy`)
	if !ok || got != `raw-legacy` {
		t.Fatalf("nil key passthrough: %q %v", got, ok)
	}
}

func TestWargameR246_RedisCrossContextBanVsVerdict(t *testing.T) {
	key := []byte("wargame-r246-fleet-secret!!!!")
	sealed := sealValue(key, "hmn:verdict:sid-Z", `{"v":"ALLOW"}`)
	if _, ok := openValue(key, "hmn:ban:ip:1.2.3.4", sealed); ok {
		t.Fatal("verdict seal must not open as ban context")
	}
}

func TestWargameR247_RedisEmptyPayloadSeal(t *testing.T) {
	key := []byte("wargame-r247-fleet-secret!!!!")
	ctx := "hmn:verdict:empty"
	sealed := sealValue(key, ctx, "")
	got, ok := openValue(key, ctx, sealed)
	if !ok || got != "" {
		t.Fatalf("empty payload round-trip: %q %v", got, ok)
	}
}

func TestWargameR248_RedisWrongKeyRejected(t *testing.T) {
	k1 := []byte("wargame-r248-key-one!!!!!!!!!!")
	k2 := []byte("wargame-r248-key-two!!!!!!!!!!")
	ctx := "hmn:verdict:sid"
	sealed := sealValue(k1, ctx, `{"v":"ALLOW"}`)
	if _, ok := openValue(k2, ctx, sealed); ok {
		t.Fatal("wrong fleet secret must reject")
	}
}

func TestWargameR249_RedisTagTruncationRejected(t *testing.T) {
	key := []byte("wargame-r249-fleet-secret!!!!")
	ctx := "hmn:verdict:sid"
	sealed := sealValue(key, ctx, `{"v":"ALLOW"}`)
	if len(sealed) < 4 {
		t.Fatal("short")
	}
	if _, ok := openValue(key, ctx, sealed[:len(sealed)/2]); ok {
		t.Fatal("truncated seal must fail")
	}
}

func TestWargameR250_AxisRedisClose(t *testing.T) {
	key := []byte("wargame-r250-fleet-secret!!!!")
	s := sealValue(key, "hmn:verdict:a", `{"v":"ALLOW"}`)
	if _, ok := openValue(key, "hmn:verdict:b", s); ok {
		t.Fatal("relocation lock")
	}
	if _, ok := openValue(key, "hmn:verdict:a", `{"v":"ALLOW"}`); ok {
		t.Fatal("unsigned lock")
	}
}
