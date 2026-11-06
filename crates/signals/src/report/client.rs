// Translated from the frozen Go wire contract by scripts/migration/contracts.
use super::BehaviorSummary;
use crate::observation::Signal;

/// UAClientHints holds parsed User-Agent Client Hints (low + high entropy).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UaClientHints {
    /// Wire field `brands`.
    #[serde(
        rename = "brands",
        deserialize_with = "crate::wire::decode::nullable_vec",
        skip_serializing_if = "crate::wire::decode::optional_vec_empty"
    )]
    pub brands: Option<Vec<UaBrand>>,
    /// Wire field `fullVersionList`.
    #[serde(
        rename = "fullVersionList",
        deserialize_with = "crate::wire::decode::nullable_vec",
        skip_serializing_if = "crate::wire::decode::optional_vec_empty"
    )]
    pub full_version_list: Option<Vec<UaBrand>>,
    /// Wire field `mobile`.
    #[serde(
        rename = "mobile",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mobile: bool,
    /// Wire field `platform`.
    #[serde(
        rename = "platform",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub platform: String,
    /// Wire field `platformVersion`.
    #[serde(
        rename = "platformVersion",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub platform_version: String,
    /// Wire field `architecture`.
    #[serde(
        rename = "architecture",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub architecture: String,
    /// Wire field `bitness`.
    #[serde(
        rename = "bitness",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub bitness: String,
    /// Wire field `model`.
    #[serde(
        rename = "model",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub model: String,
}

/// UABrand is a single brand/version pair from Client Hints.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UaBrand {
    /// Wire field `brand`.
    #[serde(
        rename = "brand",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub brand: String,
    /// Wire field `version`.
    #[serde(
        rename = "version",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub version: String,
}

/// ClientReport is the client-collected portion (L1-L4) sent to /api/collect.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClientReport {
    /// Wire field `sessionId`.
    #[serde(
        rename = "sessionId",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub session_id: String,
    /// Wire field `userAgent`.
    #[serde(
        rename = "userAgent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub user_agent: String,
    /// Wire field `uaClientHints`.
    #[serde(
        rename = "uaClientHints",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub ua_client_hints: UaClientHints,
    /// Wire field `signals`.
    #[serde(
        rename = "signals",
        deserialize_with = "crate::wire::decode::nullable_vec"
    )]
    pub signals: Option<Vec<Signal>>,
    /// Behavior carries reduced behavioral feature vectors (L4). Raw event
    /// streams are summarised client-side; see sots/03-behavioral-biometrics.md.
    #[serde(
        rename = "behavior",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub behavior: BehaviorSummary,
    /// Environment carries privacy/adblock probe results (SoT-09).
    #[serde(
        rename = "environment",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub environment: Environment,
    /// Guard carries script-injection / eval guard results (SoT-11).
    #[serde(
        rename = "guard",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub guard: Guard,
    /// Advanced carries the advanced fingerprint surfaces (SoT-14): WebRTC,
    /// voices, WebGPU, media capabilities, connection, timezone, matchMedia.
    #[serde(
        rename = "advanced",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub advanced: Advanced,
    /// FingerprintID is a stable composite hash of the client's hard-to-change
    /// surfaces (canvas/audio/webgl/fonts/timezone) used for cross-session
    /// correlation to unmask residential-proxy rotation (SoT-15).
    #[serde(
        rename = "fingerprintId",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub fingerprint_id: String,
    /// EngineVersion of the WASM detector that produced this report.
    #[serde(
        rename = "engineVersion",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub engine_version: String,
}

/// Advanced holds advanced fingerprint surfaces (SoT-14). Absences are weak
/// tells (privacy browsers / OS variation); the power is in cross-surface
/// consistency, evaluated in internal/scoring/advanced.go.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Advanced {
    /// Wire field `probed`.
    #[serde(
        rename = "probed",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub probed: bool,
    /// WebRTC / media devices.
    #[serde(
        rename = "mediaDeviceCount",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub media_device_count: i64,
    /// Wire field `hasAudioInput`.
    #[serde(
        rename = "hasAudioInput",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub has_audio_input: bool,
    /// Wire field `hasVideoInput`.
    #[serde(
        rename = "hasVideoInput",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub has_video_input: bool,
    /// Speech synthesis.
    #[serde(
        rename = "voiceCount",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub voice_count: i64,
    /// EME / Widevine DRM.
    #[serde(
        rename = "widevineSupported",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub widevine_supported: bool,
    /// WebGPU.
    #[serde(
        rename = "webgpuPresent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub web_gpu_present: bool,
    /// Wire field `webgpuVendor`.
    #[serde(
        rename = "webgpuVendor",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub web_gpu_vendor: String,
    /// Wire field `webgpuFallback`.
    #[serde(
        rename = "webgpuFallback",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub web_gpu_fallback: bool,
    /// Wire field `webglVendor`.
    #[serde(
        rename = "webglVendor",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub web_gl_vendor: String,
    /// Audio.
    #[serde(
        rename = "audioSampleRate",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub audio_sample_rate: f64,
    /// Network Information.
    #[serde(
        rename = "connectionPresent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub connection_present: bool,
    /// Wire field `connectionRtt`.
    #[serde(
        rename = "connectionRtt",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub connection_rtt: f64,
    /// Battery.
    #[serde(
        rename = "batteryPresent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub battery_present: bool,
    /// Wire field `batteryLevel`.
    #[serde(
        rename = "batteryLevel",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub battery_level: f64,
    /// Wire field `batteryCharging`.
    #[serde(
        rename = "batteryCharging",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub battery_charging: bool,
    /// Timezone / locale.
    #[serde(
        rename = "timezoneIana",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub timezone_iana: String,
    /// Wire field `timezoneOffsetMin`.
    #[serde(
        rename = "timezoneOffsetMin",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub timezone_offset: i64,
    /// Wire field `language`.
    #[serde(
        rename = "language",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub language: String,
    /// matchMedia display profile.
    #[serde(
        rename = "colorGamut",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub color_gamut: String,
    /// Wire field `dynamicRange`.
    #[serde(
        rename = "dynamicRange",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub dynamic_range: String,
    /// Wire field `pointerCoarse`.
    #[serde(
        rename = "pointerCoarse",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub pointer_coarse: bool,
    /// Wire field `maxTouchPoints`.
    #[serde(
        rename = "maxTouchPoints",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub max_touch_points: i64,
    /// Chrome-only memory API presence + new-headless tell.
    #[serde(
        rename = "perfMemoryPresent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub perf_memory_present: bool,
    /// Wire field `measureMemThrows`.
    #[serde(
        rename = "measureMemThrows",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub measure_mem_throws: bool,
    /// UA hints for consistency (mobile claim).
    #[serde(
        rename = "mobileUA",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mobile_ua: bool,
    /// WebRTC ICE residual (VPN/OpenVPN/WireGuard leak): host candidates are
    /// typically private LAN; public srflx/relay candidates should match the TCP
    /// client IP the server sees. Mismatch ⇒ tunnel exit ≠ browser's real path.
    #[serde(
        rename = "webrtcHostAddrs",
        deserialize_with = "crate::wire::decode::nullable_vec",
        skip_serializing_if = "crate::wire::decode::optional_vec_empty"
    )]
    pub web_rtc_host_addrs: Option<Vec<String>>,
    /// Wire field `webrtcPublicAddrs`.
    #[serde(
        rename = "webrtcPublicAddrs",
        deserialize_with = "crate::wire::decode::nullable_vec",
        skip_serializing_if = "crate::wire::decode::optional_vec_empty"
    )]
    pub web_rtc_public_addrs: Option<Vec<String>>,
}

/// Guard holds runtime-hardening probe results from injector.js (SoT-11).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Guard {
    /// Wire field `reported`.
    #[serde(
        rename = "reported",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub reported: bool,
    /// Wire field `evalUsed`.
    #[serde(
        rename = "evalUsed",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub eval_used: i64,
    /// Wire field `funcCtor`.
    #[serde(
        rename = "funcCtor",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub func_ctor: i64,
    /// Wire field `scriptInjected`.
    #[serde(
        rename = "scriptInjected",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub script_injected: i64,
    /// Wire field `protoPolluted`.
    #[serde(
        rename = "protoPolluted",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub proto_polluted: bool,
    /// Wire field `nativeHooked`.
    #[serde(
        rename = "nativeHooked",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub native_hooked: bool,
    /// Wire field `consoleDisabled`.
    #[serde(
        rename = "consoleDisabled",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub console_disabled: bool,
}

/// Environment holds ad-block / tracking-protection probe results (SoT-09).
/// These are treated as neutral-to-human (privacy users are common); they feed
/// consistency cross-checks and FP mitigation, never a standalone bot verdict.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Environment {
    /// probe actually ran (liveness)
    #[serde(
        rename = "probed",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub probed: bool,
    /// bait ad element/request blocked
    #[serde(
        rename = "adBlock",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub ad_block: bool,
    /// known tracker request blocked
    #[serde(
        rename = "trackingProtection",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub tracking_protection: bool,
    /// navigator.doNotTrack ("1"|"0"|"")
    #[serde(
        rename = "doNotTrack",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub do_not_track: String,
    /// navigator.globalPrivacyControl
    #[serde(rename = "gpc", deserialize_with = "crate::wire::decode::null_default")]
    pub gpc: bool,
    /// our own collector/RIT resource was blocked
    #[serde(
        rename = "collectorBlocked",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub collector_blocked: bool,
    /// navigator.brave present (Brave)
    #[serde(
        rename = "braveShields",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub brave_shields: bool,
}
