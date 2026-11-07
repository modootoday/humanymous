//! Facade request admission delegates to the public package's sole vocabulary.
use crate::Result;

pub fn check(arguments: &[String]) -> Result<()> {
    let (profile, features) = arguments.split_first().ok_or("facade profile missing")?;
    let features: Vec<_> = features.iter().map(String::as_str).collect();
    humanymous::build_profile::validate(profile, &features)
        .map_err(|_| "facade profile or feature request refused".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn supported_requests_use_the_public_admission_api() {
        assert!(check(&request(&["rust-library"])).is_ok());
        assert!(check(&request(&["rust-library", "adapter-go"])).is_ok());
    }

    #[test]
    fn missing_unknown_duplicate_and_combined_requests_are_refused() {
        for values in [
            vec![],
            vec!["unknown"],
            vec!["rust-library", "rust-library"],
            vec!["rust-library", "adapter-go", "adapter-go"],
            vec!["rust-library", "adapter-go", "native"],
            vec!["rust-library", "--all-features"],
            vec!["rust-library", ""],
        ] {
            assert!(check(&request(&values)).is_err());
        }
    }
}
