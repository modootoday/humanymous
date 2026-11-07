//! The maintenance owner contract is the only reviewed harness inventory.
use super::runtime;
use crate::{Result, require, source};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

fn contract() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../../../skills/agent-maintenance/qualification-contract.json"
    ))
    .map_err(|e| e.to_string())
}
fn inventory(owner: &Value) -> Result<Vec<(String, BTreeSet<String>)>> {
    require(
        owner["schema"] == "humanymous.agent-maintenance-qualification/0.1"
            && owner["package"] == "humanymous-agent-maintenance"
            && owner["locked"] == true
            && owner["features"] == json!([])
            && owner["runtime_qualified"] == false
            && owner["whole_product_qualified"] == false,
        "maintenance owner contract mismatch",
    )?;
    for field in ["failed", "ignored", "measured", "filtered"] {
        require(
            owner["required_summary"][field].as_u64() == Some(0),
            "maintenance summary contract mismatch",
        )?;
    }
    let mut targets = BTreeSet::new();
    let mut result = Vec::new();
    for target in owner["targets"]
        .as_array()
        .ok_or("maintenance targets absent")?
    {
        let name = target["name"]
            .as_str()
            .ok_or("maintenance target name absent")?;
        require(
            target["kind"] == "test"
                && matches!(name, "process" | "session_storage" | "product_package")
                && targets.insert(name.to_owned()),
            "maintenance target invalid or duplicate",
        )?;
        let mut names = BTreeSet::new();
        for name in target["exact_test_names"]
            .as_array()
            .ok_or("maintenance names absent")?
        {
            let name = name.as_str().ok_or("maintenance name invalid")?;
            require(
                !name.is_empty()
                    && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    && names.insert(name.to_owned()),
                "maintenance name invalid or duplicate",
            )?;
        }
        require(
            !names.is_empty() && target["expected_count"].as_u64() == Some(names.len() as u64),
            "maintenance target count mismatch",
        )?;
        result.push((name.to_owned(), names));
    }
    require(
        targets.len() == 3
            && owner["expected_total"].as_u64()
                == Some(result.iter().map(|(_, names)| names.len() as u64).sum()),
        "maintenance aggregate inventory mismatch",
    )?;
    Ok(result)
}
pub(super) fn run(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    let owner = contract()?;
    let targets = inventory(&owner)?;
    report["owner_contract_sha256"] = json!(source::file(
        &root.join("skills/agent-maintenance/qualification-contract.json")
    )?);
    let mut failed = false;
    for (target, names) in targets {
        let label = format!("maintenance-{target}");
        let list_label = format!("{label}-list");
        let base = [
            "test",
            "--locked",
            "-j1",
            "-p",
            "humanymous-agent-maintenance",
            "--test",
            &target,
        ];
        let observed: Result<u64> = (|| {
            let mut args = base.to_vec();
            args.extend(["--", "--list"]);
            require(
                runtime::invoke(root, dir, report, &list_label, &args)?,
                "maintenance discovery failed",
            )?;
            let text = std::fs::read_to_string(dir.join(format!("{list_label}.log")))
                .map_err(|e| e.to_string())?;
            let actual: BTreeSet<String> = text
                .lines()
                .filter_map(|line| line.strip_suffix(": test").map(str::to_owned))
                .collect();
            require(
                actual == names,
                "maintenance discovered inventory differs from owner contract",
            )?;
            runtime::count(&text)
        })();
        let expected = names.len() as u64;
        report["test_counts"][&target] = json!({"expected":expected,"discovered":observed.as_ref().ok(),"passed":null,"validated":false});
        // Every owner target is attempted even when another target fails.
        let outcome: Result<bool> = (|| {
            let mut args = base.to_vec();
            args.extend(["--", "--test-threads=1"]);
            let success = runtime::invoke(root, dir, report, &label, &args)?;
            let text = std::fs::read_to_string(dir.join(format!("{label}.log")))
                .map_err(|e| e.to_string())?;
            Ok(success
                && observed.as_ref().is_ok_and(|count| *count == expected)
                && runtime::execution(&text, expected))
        })();
        if outcome == Ok(true) {
            report["test_counts"][&target]["passed"] = json!(expected);
            report["test_counts"][&target]["validated"] = json!(true);
        } else {
            failed = true;
        }
    }
    require(
        !failed,
        "required independent maintenance harness incomplete",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_inventory_rejects_duplicates_and_inconsistent_totals() {
        let owner = contract().unwrap();
        assert_eq!(inventory(&owner).unwrap().len(), 3);
        let mut invalid = owner.clone();
        invalid["expected_total"] = json!(0);
        assert!(inventory(&invalid).is_err());
        let mut invalid = owner.clone();
        invalid["targets"][0]["exact_test_names"][1] =
            invalid["targets"][0]["exact_test_names"][0].clone();
        assert!(inventory(&invalid).is_err());
        let mut invalid = owner;
        invalid["required_summary"]["ignored"] = json!(1);
        assert!(inventory(&invalid).is_err());
    }
}
