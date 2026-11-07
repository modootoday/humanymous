//! Closed request admission for supported facade build entrypoints.
//! Callers must additionally bind effective Cargo features and actual artifacts.

/// Rejected profile or feature request; no caller data is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The requested product has no admitted facade build profile.
    UnknownProfile,
    /// Unknown, repeated or combined features are outside the closed profile.
    UnsupportedFeatures,
}

/// Validate an explicit feature list, before a supported entrypoint builds.
/// This does not intercept arbitrary Cargo invocations or prove feature closure.
pub fn validate(profile: &str, features: &[&str]) -> Result<(), Rejection> {
    if profile != "rust-library" {
        return Err(Rejection::UnknownProfile);
    }
    match features {
        [] | ["adapter-go"] => Ok(()),
        _ => Err(Rejection::UnsupportedFeatures),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_manifest_and_cargo_feature_vocabulary_match_admission() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../build-products.json")).unwrap();
        assert_eq!(manifest["schema"], "humanymous.build-products/1");
        assert_eq!(manifest["package"], "humanymous");
        assert_eq!(manifest["target"], "lib");
        assert_eq!(manifest["profile"], "rust-library");
        assert_eq!(
            manifest["allowed_feature_sets"],
            serde_json::json!([[], ["adapter-go"]])
        );
        for feature_set in manifest["allowed_feature_sets"].as_array().unwrap() {
            let features: Vec<_> = feature_set
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect();
            assert_eq!(validate("rust-library", &features), Ok(()));
        }
        let cargo: toml::Value = toml::from_str(include_str!("../Cargo.toml")).unwrap();
        let features = cargo["features"].as_table().unwrap();
        assert_eq!(features.len(), 2);
        assert!(features["default"].as_array().unwrap().is_empty());
        assert_eq!(
            features["adapter-go"].as_array().unwrap(),
            &[toml::Value::String("dep:humanymous-adapter-go".into())]
        );
        assert_eq!(
            cargo["dependencies"]["humanymous-adapter-go"]["optional"].as_bool(),
            Some(true)
        );
        assert!(
            !cargo["dev-dependencies"]
                .as_table()
                .unwrap()
                .contains_key("humanymous-adapter-go")
        );
        assert_eq!(cargo["package"]["autobins"].as_bool(), Some(false));
        assert!(cargo.get("bin").is_none());
        assert!(cargo.get("lib").is_none());
    }

    #[test]
    fn unknown_profiles_features_and_duplicate_requests_fail_closed() {
        assert_eq!(
            validate("node-extension", &[]),
            Err(Rejection::UnknownProfile)
        );
        for features in [
            vec!["humanymous-go-adapter"],
            vec!["humanymous-adapter-go"],
            vec!["adapter-go", "adapter-go"],
            vec!["adapter-go", "native"],
            vec![""],
            vec!["default"],
        ] {
            assert_eq!(
                validate("rust-library", &features),
                Err(Rejection::UnsupportedFeatures)
            );
        }
    }
}
