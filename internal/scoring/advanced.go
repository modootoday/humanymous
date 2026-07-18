package scoring

import (
	"strings"

	"github.com/modootoday/humanymous/internal/signals"
)

// advanced.go derives L2 advanced-fingerprint signals from the SoT-14 probe.
// Design invariant (SoT-14 §13): single-API absences are WEAK tells (privacy
// browsers / OS variation), so most are SUSPICIOUS with modest confidence; the
// strong signals are cross-surface CONTRADICTIONS (WebGL↔WebGPU vendor, mobile
// UA vs desktop profile). Headful real browsers (the automated "human" baseline
// and the headful-frontier bots) pass these; headless bots trip several.

// AdvancedSignals evaluates the advanced fingerprint surfaces.
func AdvancedSignals(a signals.Advanced, ua string) []signals.Signal {
	if !a.Probed {
		return nil
	}
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, conf float64, notes string) {
		out = append(out, signals.New(id, val, v, conf, signals.SourceJS, notes))
	}

	claimsChrome := strings.Contains(strings.ToLower(ua), "chrome")
	desktop := !a.MobileUA

	// AudioContext 24000 Hz — a strong, low-FP headless tell.
	if a.AudioSampleRate == 24000 {
		add("l2.adv.audio_24k", a.AudioSampleRate, signals.VerdictBot, 0.8,
			"AudioContext sampleRate 24000 (no audio hardware)")
	}
	// No speech voices — headless (FP: Linux desktop). Weak.
	if a.VoiceCount == 0 {
		add("l2.adv.no_voices", 0, signals.VerdictSuspicious, 0.5, "no speech-synthesis voices")
	}
	// No media devices under a desktop UA — headless container (FP: no webcam on
	// many real desktops/VMs, so low confidence).
	if desktop && a.MediaDeviceCount == 0 {
		add("l2.adv.no_media_devices", 0, signals.VerdictSuspicious, 0.35, "enumerateDevices empty on desktop")
	}
	// Widevine absent under a Chrome UA — plain Chromium (Playwright default).
	if claimsChrome && !a.WidevineSupported {
		add("l2.adv.no_widevine", false, signals.VerdictSuspicious, 0.6, "Widevine unsupported under Chrome UA")
	}
	// connection.rtt == 0 under desktop — headless/datacenter.
	if a.ConnectionPresent && a.ConnectionRTT == 0 && desktop {
		add("l2.adv.rtt_zero", 0, signals.VerdictSuspicious, 0.4, "connection.rtt 0")
	}
	// measureUserAgentSpecificMemory throws — new-headless discriminator.
	if a.MeasureMemThrows {
		add("l2.adv.measure_mem_throws", true, signals.VerdictSuspicious, 0.5, "measureUserAgentSpecificMemory throws")
	}

	// --- strong cross-surface contradictions ---
	// WebGL vendor vs WebGPU vendor must describe the same physical GPU.
	if a.WebGPUPresent && a.WebGPUVendor != "" && a.WebGLVendor != "" {
		if !gpuVendorsAgree(a.WebGLVendor, a.WebGPUVendor) {
			add("l2.adv.webgpu_mismatch",
				map[string]string{"webgl": a.WebGLVendor, "webgpu": a.WebGPUVendor},
				signals.VerdictBot, 0.9, "WebGL vs WebGPU GPU vendor mismatch (spoof)")
		}
	}
	// Mobile UA but desktop-class profile (no touch + coarse-pointer false).
	if a.MobileUA && a.MaxTouchPoints == 0 && !a.PointerCoarse {
		add("l2.adv.mobile_inconsistent",
			map[string]any{"touch": a.MaxTouchPoints, "coarse": a.PointerCoarse},
			signals.VerdictBot, 0.8, "mobile UA with desktop pointer/touch profile")
	}

	return out
}

// gpuVendorsAgree reports whether two GPU vendor strings plausibly describe the
// same GPU family (substring/keyword match, case-insensitive).
func gpuVendorsAgree(webgl, webgpu string) bool {
	a, b := strings.ToLower(webgl), strings.ToLower(webgpu)
	for _, kw := range []string{"nvidia", "amd", "ati", "intel", "apple", "qualcomm", "adreno", "mali", "arm", "google", "microsoft", "swiftshader", "llvmpipe"} {
		if strings.Contains(a, kw) && strings.Contains(b, kw) {
			return true
		}
	}
	// If neither string contains a known keyword, don't claim a mismatch.
	if !containsAnyGPU(a) || !containsAnyGPU(b) {
		return true
	}
	return false
}

func containsAnyGPU(s string) bool {
	for _, kw := range []string{"nvidia", "amd", "ati", "intel", "apple", "qualcomm", "adreno", "mali", "arm", "google", "microsoft", "swiftshader", "llvmpipe"} {
		if strings.Contains(s, kw) {
			return true
		}
	}
	return false
}
