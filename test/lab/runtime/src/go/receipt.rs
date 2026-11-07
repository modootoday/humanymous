//! Qualify complete measured Go ledgers with independently provisioned metadata.
use humanymous::adapters::go::{ExpectedTuple, Host, Receipt};
use serde_json::{Value, json};
use std::io;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    Ok(value[key]
        .as_str()
        .ok_or("expected immutable Go tuple field")?)
}
pub fn qualify() -> Result<()> {
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut io::Read::take(io::stdin(), 1_048_577), &mut bytes)?;
    let value = humanymous_test_support::json::decode_strict_json(&bytes, 1_048_576)?;
    let receipt: Receipt = serde_json::from_value(value["receipt"].clone())?;
    let expected = &value["expected"];
    let tuple = ExpectedTuple {
        host: Host::NetHttp,
        source: text(expected, "source")?,
        image: text(expected, "image")?,
        fixture: text(expected, "fixture")?,
        configuration: text(expected, "configuration")?,
        engine: text(expected, "engine")?,
        host_version: text(expected, "host_version")?,
        go_version: text(expected, "go_version")?,
        os: text(expected, "os")?,
        arch: text(expected, "arch")?,
    };
    let qualified = receipt.qualifies(&tuple);
    println!(
        "{}",
        json!({"schema":"humanymous.go-lab-qualification/1","qualified_component":qualified,"profile":receipt.profile,"scheduled":receipt.outcomes.len(),"full_product_qualified":false})
    );
    if !qualified {
        return Err("actual Go bounded HTTP qualification failed".into());
    }
    Ok(())
}
pub fn evaluate() -> Result<()> {
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut io::Read::take(io::stdin(), 1_048_577), &mut bytes)?;
    let value = humanymous_test_support::json::decode_strict_json(&bytes, 1_048_576)?;
    let observation: humanymous::adapters::go::scenario::CaseObservation =
        serde_json::from_value(value)?;
    let result = match humanymous::adapters::go::scenario::evaluate(&observation) {
        Ok(outcome) => {
            json!({"schema":"humanymous.go-case-evaluation/1","status":"passed","outcome":outcome})
        }
        Err(error) => {
            json!({"schema":"humanymous.go-case-evaluation/1","status":"failed","case":observation.case,"error_code":error.code,"assertions":error.assertions})
        }
    };
    println!("{result}");
    Ok(())
}
