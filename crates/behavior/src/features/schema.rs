use sha2::{Digest, Sha256};

/// Version of the ordered feature layout.
pub const FEATURE_SCHEMA_VERSION: &str = "bv1";
/// Fixed tensor dimension.
pub const FEATURE_DIM: usize = 27;
/// Ordered feature names; reordering changes model compatibility.
pub const FEATURE_NAMES: [&str; FEATURE_DIM] = [
    "mouse.samples.log",
    "mouse.mean_curvature",
    "mouse.velocity_std",
    "mouse.accel_entropy",
    "mouse.straight_frac",
    "mouse.pause_count.log",
    "mouse.max_teleport.log",
    "mouse.mean_jerk",
    "mouse.coalesced_ratio",
    "key.keystrokes.log",
    "key.mean_dwell_ms.log",
    "key.dwell_std_ms.log",
    "key.mean_flight_ms.log",
    "key.flight_std_ms.log",
    "key.paste_events.log",
    "key.zero_dwell_frac",
    "event.total.log",
    "event.untrusted_frac",
    "event.no_move_clicks.log",
    "event.synthetic_flags.log",
    "event.click_count.log",
    "event.long_gap_count.log",
    "event.burst_var_ms.log",
    "event.min_reaction_ms.log",
    "window.duration_s.log",
    "presence.has_pointer",
    "presence.has_key",
];
/// A fixed-size, ordered training/inference tensor.
pub type FeatureVector = [f32; FEATURE_DIM];

/// First twelve SHA-256 bytes of version, NUL and comma-separated feature names.
pub fn schema_hash() -> String {
    let mut digest = Sha256::new();
    digest.update(FEATURE_SCHEMA_VERSION.as_bytes());
    digest.update([0]);
    digest.update(FEATURE_NAMES.join(",").as_bytes());
    hex::encode(&digest.finalize()[..12])
}
