//! Effective posture (SoT-39 §2.2) and its engine mapping, equal to the oracle
//! `Resolve` and `gate.ConfigureEngineFromEffective`.
use super::wire::Json;
use super::{
    KNOWN_GATES, Mode, Overlay, OverlayEncodeError, hash_config_version, scoring_defaults,
};
use humanymous_core::scoring::{engine::Engine, policy::Policy};
use std::collections::BTreeMap;

/// Resolved posture read by operators and the engine.
#[derive(Clone, Debug, PartialEq)]
pub struct Effective {
    /// `cfg-…` version binding posture, routes, rate and overlay digest.
    pub config_version: String,
    /// Monitor posture; forced on by the kill switch.
    pub global_monitor: bool,
    /// Boot kill switch.
    pub kill_switch: bool,
    /// Every known gate plus overlay entries.
    pub gates: BTreeMap<String, Mode>,
    /// Hard-rule overrides only; a missing rule enforces.
    pub hard_rules: BTreeMap<String, Mode>,
    /// Challenge threshold.
    pub challenge_at: f64,
    /// Deny threshold.
    pub deny_at: f64,
    /// Per-layer cap.
    pub layer_cap: f64,
    /// Weight multipliers; empty means none (the oracle's nil map).
    pub weights: BTreeMap<String, f64>,
    /// Residual class modes; empty means code defaults (the oracle's nil map).
    pub net_policy: BTreeMap<String, Mode>,
    /// Route presets.
    pub routes: BTreeMap<String, String>,
    /// Rate window in seconds.
    pub rate_window_sec: i64,
    /// Soft rate threshold.
    pub rate_soft: i64,
    /// Hard rate threshold.
    pub rate_hard: i64,
    /// Applied overlay ID.
    pub overlay_id: String,
    /// Applied overlay status.
    pub overlay_status: String,
    /// True when no overlay was applied.
    pub empty_overlay: bool,
}

/// Boot configuration slice the resolver needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootInput {
    /// Boot global monitor posture.
    pub global_monitor: bool,
    /// Boot kill switch.
    pub kill_switch: bool,
    /// Boot route presets.
    pub routes: BTreeMap<String, String>,
    /// Boot rate window in seconds.
    pub rate_window_sec: i64,
    /// Boot soft rate threshold.
    pub rate_soft: i64,
    /// Boot hard rate threshold.
    pub rate_hard: i64,
    /// Configuration-version HMAC key.
    pub hmac_key: Vec<u8>,
}

/// Build the effective posture from boot input and an optional active overlay.
/// Only `active` or empty status applies, but any supplied overlay with an ID
/// still contributes its digest to the configuration version, as in the oracle.
pub fn resolve(boot: &BootInput, active: Option<&Overlay>) -> Effective {
    let (challenge_at, deny_at, layer_cap) = scoring_defaults();
    let mut effective = Effective {
        config_version: String::new(),
        global_monitor: boot.global_monitor,
        kill_switch: boot.kill_switch,
        gates: KNOWN_GATES
            .iter()
            .map(|gate| ((*gate).to_owned(), Mode::Enforce))
            .collect(),
        hard_rules: BTreeMap::new(),
        challenge_at,
        deny_at,
        layer_cap,
        weights: BTreeMap::new(),
        net_policy: BTreeMap::new(),
        routes: boot.routes.clone(),
        rate_window_sec: boot.rate_window_sec,
        rate_soft: boot.rate_soft,
        rate_hard: boot.rate_hard,
        overlay_id: String::new(),
        overlay_status: String::new(),
        empty_overlay: true,
    };
    if let Some(overlay) = active.filter(|overlay| matches!(overlay.status.as_str(), "active" | ""))
    {
        effective.empty_overlay = false;
        effective.overlay_id = overlay.overlay_id.clone();
        effective.overlay_status = if overlay.status.is_empty() {
            "active".into()
        } else {
            overlay.status.clone()
        };
        effective.gates.extend(overlay.gates.clone());
        effective.hard_rules.extend(overlay.hard_rules.clone());
        if let Some(scoring) = &overlay.scoring {
            effective.challenge_at = scoring.challenge_at.unwrap_or(effective.challenge_at);
            effective.deny_at = scoring.deny_at.unwrap_or(effective.deny_at);
            effective.layer_cap = scoring.layer_cap.unwrap_or(effective.layer_cap);
        }
        effective.weights = overlay.weight_multipliers.clone();
        effective.net_policy = overlay.net_policy.clone();
        effective.routes.extend(overlay.routes.clone());
        if let Some(rate) = &overlay.rate_limit {
            effective.rate_window_sec = rate.window_sec.unwrap_or(effective.rate_window_sec);
            effective.rate_soft = rate.soft.unwrap_or(effective.rate_soft);
            effective.rate_hard = rate.hard.unwrap_or(effective.rate_hard);
        }
        if let Some(monitor) = overlay.global_monitor {
            effective.global_monitor = monitor;
        }
    }
    // The kill switch always wins: enforcement is suppressed.
    if boot.kill_switch {
        effective.global_monitor = true;
    }
    effective.config_version = hash_config_version(
        &boot.hmac_key,
        effective.global_monitor,
        boot.kill_switch,
        &effective.routes,
        effective.rate_window_sec,
        effective.rate_soft,
        effective.rate_hard,
        active,
    );
    effective
}

impl Effective {
    /// Hard-rule mode; enforce when not overridden.
    pub fn hr_mode(&self, id: &str) -> Mode {
        self.hard_rules.get(id).cloned().unwrap_or(Mode::Enforce)
    }
    /// Gate mode; enforce when unknown.
    pub fn gate_mode(&self, id: &str) -> Mode {
        self.gates.get(id).cloned().unwrap_or(Mode::Enforce)
    }
    /// Go `json.Marshal` of the oracle `Effective` struct.
    pub fn to_json(&self) -> Result<Vec<u8>, OverlayEncodeError> {
        let mut fields = vec![
            (
                "configVersion".into(),
                Json::Str(self.config_version.clone()),
            ),
            ("globalMonitor".into(), Json::Bool(self.global_monitor)),
            ("killSwitch".into(), Json::Bool(self.kill_switch)),
            ("gates".into(), Json::modes(&self.gates)),
            ("hardRules".into(), Json::modes(&self.hard_rules)),
            ("challengeAt".into(), Json::Float(self.challenge_at)),
            ("denyAt".into(), Json::Float(self.deny_at)),
            ("layerCap".into(), Json::Float(self.layer_cap)),
        ];
        if !self.weights.is_empty() {
            fields.push(("weightMultipliers".into(), Json::floats(&self.weights)));
        }
        if !self.net_policy.is_empty() {
            fields.push(("netPolicy".into(), Json::modes(&self.net_policy)));
        }
        fields.extend([
            ("routes".into(), Json::texts(&self.routes)),
            ("rateWindowSec".into(), Json::Int(self.rate_window_sec)),
            ("rateSoft".into(), Json::Int(self.rate_soft)),
            ("rateHard".into(), Json::Int(self.rate_hard)),
        ]);
        if !self.overlay_id.is_empty() {
            fields.push(("overlayId".into(), Json::Str(self.overlay_id.clone())));
        }
        if !self.overlay_status.is_empty() {
            fields.push((
                "overlayStatus".into(),
                Json::Str(self.overlay_status.clone()),
            ));
        }
        fields.push(("emptyOverlay".into(), Json::Bool(self.empty_overlay)));
        Json::Object(fields).encode(false).map(String::into_bytes)
    }
}

/// Configure the shared scoring engine from the effective posture (the oracle's
/// `ConfigureEngineFromEffective`). The frozen default policy is kept for an
/// empty overlay; otherwise only the version label gains `+overlay`. The
/// engine's model runtime is left untouched.
pub fn configure_engine(engine: &mut Engine, effective: &Effective) {
    let mut policy = Policy {
        challenge_at: effective.challenge_at,
        deny_at: effective.deny_at,
        layer_cap: effective.layer_cap,
        ..Policy::default()
    };
    if !effective.empty_overlay {
        policy.version = "1.0.0+overlay".into();
    }
    engine.policy = policy;
    engine.rule_modes = effective
        .hard_rules
        .iter()
        .map(|(id, mode)| (id.clone(), mode.as_str().to_owned()))
        .collect();
    engine.weight_multipliers = effective.weights.clone();
    engine.net_policy = (!effective.net_policy.is_empty()).then(|| {
        effective
            .net_policy
            .iter()
            .map(|(id, mode)| (id.clone(), mode.as_str().to_owned()))
            .collect()
    });
}
