package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// T4 promotion doctrine (user-directed):
// - Honest ceiling: fully coherent spoof may still SCORE ALLOW (detection limit).
// - Promotion: any residual crack in that coherence must NOT ALLOW — raise cost via
//   hard rules / advanced residuals / (at Gate) attestation floor pricing.
// This is not a "100% solved T4" claim; it is active defense against near-ceiling bots.

func coherentLike(advanced signals.Advanced) *signals.SessionReport {
	ua := "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
	r := base(ua, []signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	r.Client.UAClientHints = signals.UAClientHints{
		Platform: "Windows",
		Brands:   []signals.UABrand{{Brand: "Chromium", Version: "126"}, {Brand: "Google Chrome", Version: "126"}},
	}
	r.Client.Advanced = advanced
	r.Client.EngineVersion = "wasm-1.0.0"
	return r
}

func TestWargameR340_T4HonestCeilingMayAllow(t *testing.T) {
	// Full coherent advanced surface — detection ceiling remains ALLOW-capable.
	r := coherentLike(signals.Advanced{
		Probed: true, MediaDeviceCount: 3, VoiceCount: 200, WidevineSupported: true,
		WebGPUPresent: true, WebGPUVendor: "nvidia",
		WebGLVendor: "NVIDIA Corporation / NVIDIA GeForce RTX 3080",
		AudioSampleRate: 48000, ConnectionPresent: true, ConnectionRTT: 50,
		MaxTouchPoints: 0,
	})
	v := scoreOf(t, r)
	if v.Verdict == VerdictDeny {
		t.Fatalf("honest T4 coherent must not hard-DENY without residual: %s/%s risk=%.1f",
			v.Verdict, v.HardRuleFired, v.RiskScore)
	}
	// ALLOW or soft CHALLENGE both acceptable for pure coherent; document outcome.
	t.Logf("T4 coherent posture: %s rule=%s risk=%.1f", v.Verdict, v.HardRuleFired, v.RiskScore)
}

func TestWargameR341_NearCeilingAudio24kPromotedBlock(t *testing.T) {
	r := coherentLike(signals.Advanced{
		Probed: true, MediaDeviceCount: 2, VoiceCount: 80, WidevineSupported: true,
		WebGPUPresent: true, WebGPUVendor: "nvidia",
		WebGLVendor: "NVIDIA Corporation / NVIDIA GeForce RTX 3080",
		AudioSampleRate: 24000, // residual
		ConnectionPresent: true, ConnectionRTT: 40,
	})
	v := scoreOf(t, r)
	if v.Verdict == VerdictAllow {
		t.Fatalf("audio 24k residual must not ALLOW (T4 promotion): risk=%.1f top=%v",
			v.RiskScore, v.TopContributors)
	}
}

func TestWargameR342_NearCeilingNoWidevinePromotedBlock(t *testing.T) {
	r := coherentLike(signals.Advanced{
		Probed: true, MediaDeviceCount: 0, VoiceCount: 0, WidevineSupported: false,
		WebGPUPresent: true, WebGPUVendor: "google", WebGLVendor: "Google Inc. (Google)",
		AudioSampleRate: 48000, ConnectionPresent: true, ConnectionRTT: 30,
	})
	v := scoreOf(t, r)
	if v.Verdict == VerdictAllow {
		t.Fatalf("Chrome UA without Widevine/media must not ALLOW: risk=%.1f", v.RiskScore)
	}
}

func TestWargameR343_NearCeilingWebGPUMismatchPromotedBlock(t *testing.T) {
	r := coherentLike(signals.Advanced{
		Probed: true, MediaDeviceCount: 2, VoiceCount: 50, WidevineSupported: true,
		WebGPUPresent: true, WebGPUVendor: "intel",
		WebGLVendor: "NVIDIA Corporation / NVIDIA GeForce RTX 3080",
		AudioSampleRate: 48000, ConnectionPresent: true, ConnectionRTT: 40,
	})
	v := scoreOf(t, r)
	if v.Verdict == VerdictAllow {
		t.Fatalf("GPU family mismatch must not ALLOW: risk=%.1f", v.RiskScore)
	}
}

func TestWargameR344_NearCeilingProxyRotationStillHR19(t *testing.T) {
	// Even a coherent client surface cannot launder server proxy_rotation.
	r := coherentLike(signals.Advanced{
		Probed: true, MediaDeviceCount: 3, VoiceCount: 100, WidevineSupported: true,
		WebGPUPresent: true, WebGPUVendor: "nvidia",
		WebGLVendor: "NVIDIA Corporation / NVIDIA GeForce RTX 3080",
		AudioSampleRate: 48000, ConnectionPresent: true, ConnectionRTT: 50,
	})
	r.Network.Signals = append(r.Network.Signals, bot("l5.correlation.proxy_rotation"))
	if v := scoreOf(t, r); v.HardRuleFired != "HR-19" {
		t.Fatalf("proxy farm under coherent surface must HR-19, got %s/%s", v.HardRuleFired, v.Verdict)
	}
}

func TestWargameR345_T4PromotionDoctrineSummary(t *testing.T) {
	// Compact lock used by CI/wargame: residuals blocked; pure coherent not hard-DENY-only.
	TestWargameR341_NearCeilingAudio24kPromotedBlock(t)
	TestWargameR342_NearCeilingNoWidevinePromotedBlock(t)
	TestWargameR344_NearCeilingProxyRotationStillHR19(t)
}
