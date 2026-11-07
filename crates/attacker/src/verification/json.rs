//! Report-specific budget and safe diagnostics over the shared syntax boundary.
use humanymous_defender::lab::LabError;
use humanymous_test_support::json::{StrictJsonError, decode_strict_json};
use serde_json::Value;

pub(super) fn decode(bytes: &[u8]) -> Result<Value, LabError> {
    decode_strict_json(bytes, 4_194_304).map_err(|error| match error {
        StrictJsonError::TooLarge => LabError("report size budget"),
        StrictJsonError::Invalid => LabError("report JSON syntax or duplicate object key"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_budget_and_invalid_input_keep_safe_diagnostics() {
        assert_eq!(
            decode(&vec![b' '; 4_194_305]).unwrap_err().to_string(),
            "report size budget"
        );
        assert_eq!(
            decode(br#"{"private-key":0,"private-key":1}"#)
                .unwrap_err()
                .to_string(),
            "report JSON syntax or duplicate object key"
        );
    }
}
