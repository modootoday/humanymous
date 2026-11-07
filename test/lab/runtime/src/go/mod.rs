//! Actual untouched Go-host conformance actors; no native Go SDK or human claim.
mod backend;
mod client;
mod gate;
mod receipt;
mod relay;
mod response;
use humanymous_go_adapter::{Deployment, Host};
use humanymous_runtime::configuration::Configuration;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn deployment(value: serde_json::Value) -> Result<Configuration> {
    let configuration = serde_json::from_value(value)?;
    // Consume the actual Go adapter contract before the existing runtime host.
    Ok(Deployment::new(Host::NetHttp, configuration)
        .map_err(|_| "Go deployment configuration rejected")?
        .into_configuration())
}

fn configuration_value(value: serde_json::Value, forged: bool) -> Result<serde_json::Value> {
    let mut expected = humanymous_test_support::json::decode_strict_json(
        include_bytes!("../../../go-configuration.json"),
        65536,
    )?;
    expected["origin"] = value["origin"].clone();
    expected["audit"]["keys"]["passphrase"] = value["audit"]["keys"]["passphrase"].clone();
    let mut value = expected;
    let contract = humanymous_test_support::json::decode_strict_json(
        include_bytes!("../../../../../adapters/go/pilot-contract.json"),
        65536,
    )?;
    value["routes"] = contract["baseline_routes"].clone();
    if forged {
        value["routes"][0]["required"]["raw_tls"] = serde_json::json!(true);
    }
    Ok(value)
}
fn configuration(value: serde_json::Value, forged: bool) -> Result<Configuration> {
    deployment(configuration_value(value, forged)?)
}

pub fn client() -> Result<()> {
    client::run()
}
pub fn qualify() -> Result<()> {
    receipt::qualify()
}
pub fn gate() -> Result<()> {
    gate::run()
}
pub fn count() -> Result<()> {
    gate::count()
}
pub fn backend() -> Result<()> {
    backend::run()
}
pub fn backend_stop() -> Result<()> {
    backend::stop()
}
pub fn evaluate() -> Result<()> {
    receipt::evaluate()
}
pub fn client_release() -> Result<()> {
    client::release()
}
