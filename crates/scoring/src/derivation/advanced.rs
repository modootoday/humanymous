use crate::consistency::user_agent::lowercase;
use humanymous_signals::{
    observation::{Signal, Source, Verdict},
    report::Advanced,
};
use serde_json::json;

const GPU_KEYWORDS: [&str; 13] = [
    "nvidia",
    "amd",
    "ati",
    "intel",
    "apple",
    "qualcomm",
    "adreno",
    "mali",
    "arm",
    "google",
    "microsoft",
    "swiftshader",
    "llvmpipe",
];

fn vendors_agree(webgl: &str, webgpu: &str) -> bool {
    let (first, second) = (lowercase(webgl), lowercase(webgpu));
    GPU_KEYWORDS
        .iter()
        .any(|keyword| first.contains(keyword) && second.contains(keyword))
        || !GPU_KEYWORDS.iter().any(|keyword| first.contains(keyword))
        || !GPU_KEYWORDS.iter().any(|keyword| second.contains(keyword))
}

/// Derive advanced observations using the existing probe predicates and confidences.
pub fn advanced_signals(advanced: &Advanced, user_agent: &str) -> Vec<Signal> {
    if !advanced.probed {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut add = |id, value, verdict, confidence, notes| {
        result.push(Signal::new(
            id,
            value,
            verdict,
            confidence,
            Source::Js,
            notes,
        ))
    };
    let claims_chrome = lowercase(user_agent).contains("chrome");
    let desktop = !advanced.mobile_ua;
    if advanced.audio_sample_rate == 24000.0 {
        add(
            "l2.adv.audio_24k",
            json!(advanced.audio_sample_rate),
            Verdict::Bot,
            0.8,
            "AudioContext sampleRate 24000 (no audio hardware)",
        );
    }
    if advanced.voice_count == 0 {
        add(
            "l2.adv.no_voices",
            json!(0),
            Verdict::Suspicious,
            0.5,
            "no speech-synthesis voices",
        );
    }
    if desktop && advanced.media_device_count == 0 {
        add(
            "l2.adv.no_media_devices",
            json!(0),
            Verdict::Suspicious,
            0.35,
            "enumerateDevices empty on desktop",
        );
    }
    if claims_chrome && !advanced.widevine_supported {
        add(
            "l2.adv.no_widevine",
            json!(false),
            Verdict::Suspicious,
            0.6,
            "Widevine unsupported under Chrome UA",
        );
    }
    if advanced.connection_present && advanced.connection_rtt == 0.0 && desktop {
        add(
            "l2.adv.rtt_zero",
            json!(0),
            Verdict::Suspicious,
            0.4,
            "connection.rtt 0",
        );
    }
    if advanced.measure_mem_throws {
        add(
            "l2.adv.measure_mem_throws",
            json!(true),
            Verdict::Suspicious,
            0.5,
            "measureUserAgentSpecificMemory throws",
        );
    }
    if advanced.web_gpu_present
        && !advanced.web_gpu_vendor.is_empty()
        && !advanced.web_gl_vendor.is_empty()
        && !vendors_agree(&advanced.web_gl_vendor, &advanced.web_gpu_vendor)
    {
        add(
            "l2.adv.webgpu_mismatch",
            json!({"webgl":advanced.web_gl_vendor,"webgpu":advanced.web_gpu_vendor}),
            Verdict::Bot,
            0.9,
            "WebGL vs WebGPU GPU vendor mismatch (spoof)",
        );
    }
    if advanced.mobile_ua && advanced.max_touch_points == 0 && !advanced.pointer_coarse {
        add(
            "l2.adv.mobile_inconsistent",
            json!({"touch":advanced.max_touch_points,"coarse":advanced.pointer_coarse}),
            Verdict::Bot,
            0.8,
            "mobile UA with desktop pointer/touch profile",
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_gpu_vendors_do_not_invent_a_contradiction() {
        assert!(vendors_agree("unknown", "NVIDIA"));
        assert!(vendors_agree("NVIDIA renderer", "nvidia"));
        assert!(!vendors_agree("Intel", "NVIDIA"));
        assert!(advanced_signals(&Advanced::default(), "Chrome").is_empty());
    }
}
