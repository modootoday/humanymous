//! Provider execution prerequisites only. No resource/API mutation is implemented.
use crate::docker::Result;
use serde_json::{Value, json};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn validate(config: &Value, now: u64) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !matches!(
        config["provider"].as_str(),
        Some("cloudflare" | "aws-lightsail" | "aws-ec2")
    ) {
        missing.push("supported-provider");
    }
    if config["explicit_opt_in"] != true {
        missing.push("explicit-opt-in");
    }
    let host = config["owned_hostname"].as_str().unwrap_or("");
    if host.is_empty()
        || !host.contains('.')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
        || host.starts_with('.')
        || host.ends_with('.')
    {
        missing.push("exact-owned-hostname");
    }
    if config["owned_installation_confirmed"] != true {
        missing.push("owned-installation-confirmation");
    }
    if !config["budget_usd"]
        .as_f64()
        .is_some_and(|n| n.is_finite() && n > 0.0)
    {
        missing.push("explicit-positive-budget");
    }
    if !config["expires_unix"]
        .as_u64()
        .is_some_and(|n| n > now && n <= now + 3600)
    {
        missing.push("bounded-expiry");
    }
    if config["external_janitor_ready"] != true {
        missing.push("external-cancellation-janitor");
    }
    if config["qualified_gate_manifest"]
        .as_str()
        .unwrap_or("")
        .is_empty()
    {
        missing.push("qualified-gate-source-manifest");
    }
    if config["provider"] == "cloudflare" {
        if config["allowed_domains"] != json!([host]) {
            missing.push("exact-browser-domain-allowlist");
        }
        for name in ["access_team", "access_audience"] {
            if config[name].as_str().unwrap_or("").is_empty() {
                missing.push("access-team-and-audience");
            }
        }
    }
    missing
}
pub fn run(args: &[String]) -> Result<i32> {
    if args.len() != 1 {
        return Err("provider-preflight requires one local manifest; never credentials".into());
    }
    let config: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut missing = validate(&config, now);
    for variable in match config["provider"].as_str() {
        Some("cloudflare") => vec![
            "CLOUDFLARE_API_TOKEN",
            "HM_TUNNEL_TOKEN",
            "HM_ACCESS_CLIENT_ID",
            "HM_ACCESS_CLIENT_SECRET",
        ],
        _ => vec!["AWS_PROFILE"],
    } {
        if std::env::var_os(variable).is_none_or(|value| value.is_empty()) {
            missing.push(variable);
        }
    }
    // Satisfying these checks never grants provisioning permission or runtime parity.
    println!(
        "{}",
        json!({"schema":"humanymous.provider-preflight/1","provider":config["provider"],"missing":missing,"account_mutated":false,"deployment_implemented":false,"execution_ready":false,"reason":"full Gate and provider execution/cleanup qualification remain unavailable; Cloudflare account operations must use cf CLI without Wrangler config"})
    );
    Ok(2)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opt_in_budget_expiry_and_exact_domains_are_required() {
        let valid = json!({"provider":"cloudflare","explicit_opt_in":true,"owned_hostname":"owned.invalid","owned_installation_confirmed":true,"budget_usd":1,"expires_unix":200,"external_janitor_ready":true,"qualified_gate_manifest":"digest","allowed_domains":["owned.invalid"],"access_team":"team","access_audience":"audience"});
        assert!(validate(&valid, 100).is_empty());
        for (key, value) in [
            ("explicit_opt_in", json!(false)),
            ("budget_usd", json!(0)),
            ("expires_unix", json!(100)),
            ("allowed_domains", json!(["*.invalid"])),
            ("external_janitor_ready", json!(false)),
        ] {
            let mut bad = valid.clone();
            bad[key] = value;
            assert!(!validate(&bad, 100).is_empty());
        }
    }
}
