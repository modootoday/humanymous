use super::{MlpBundle, Prediction, Predictor};
use humanymous_behavior::features::FEATURE_DIM;
use std::path::Path;

/// Loaded immutable predictor; construction validates all indexable shapes.
#[derive(Debug, Clone)]
pub struct Mlp {
    bundle: MlpBundle,
}

impl Mlp {
    /// Validate an in-memory artifact before making it executable.
    pub fn from_bundle(bundle: MlpBundle) -> Result<Self, String> {
        bundle.validate()?;
        Ok(Self { bundle })
    }
    /// Decode and validate existing JSON artifact bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let bundle = serde_json::from_slice(bytes)
            .map_err(|error| format!("mlserve: bundle parse: {error}"))?;
        Self::from_bundle(bundle)
    }
    /// Load an artifact from disk; missing or malformed files return an error.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        Self::from_json(&std::fs::read(path).map_err(|error| error.to_string())?)
    }
}

impl Predictor for Mlp {
    fn predict(&self, features: &[f32]) -> Prediction {
        if features.len() != FEATURE_DIM {
            return Prediction::abstained();
        }
        let bundle = &self.bundle;
        let mut standardized = [0.0_f32; FEATURE_DIM];
        for (index, feature) in features.iter().enumerate() {
            let std = if bundle.std[index] == 0.0 {
                1.0
            } else {
                bundle.std[index]
            };
            standardized[index] = (*feature - bundle.mean[index]) / std;
        }
        let mut logit = bundle.b2;
        for hidden in 0..bundle.hidden as usize {
            let mut accumulator = bundle.b1[hidden];
            for (weight, input) in bundle.w1[hidden].iter().zip(standardized.iter()) {
                accumulator += weight * input;
            }
            if accumulator < 0.0 {
                accumulator = 0.0;
            }
            logit += bundle.w2[hidden] * accumulator;
        }
        let calibrated = bundle.cal_a * logit + bundle.cal_b;
        let probability = (1.0_f64 / (1.0 + f64::from(-calibrated).exp())) as f32;
        if probability.is_nan() || !(0.0..=1.0).contains(&probability) {
            return Prediction::abstained();
        }
        Prediction {
            p_bot: probability,
            abstain: false,
        }
    }
    fn schema_hash(&self) -> &str {
        &self.bundle.schema_hash
    }
    fn bundle_version(&self) -> &str {
        &self.bundle.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_behavior::features::schema_hash;

    fn bundle() -> MlpBundle {
        let mut row = vec![0.0; FEATURE_DIM];
        row[0] = 2.0;
        MlpBundle {
            version: "test".into(),
            schema_hash: schema_hash(),
            feature_dim: FEATURE_DIM as i64,
            hidden: 1,
            mean: vec![0.0; FEATURE_DIM],
            std: vec![1.0; FEATURE_DIM],
            w1: vec![row],
            b1: vec![0.0],
            w2: vec![1.0],
            cal_a: 1.0,
            ..Default::default()
        }
    }
    #[test]
    fn forward_math_shape_errors_and_nonfinite_inputs() {
        let model = Mlp::from_bundle(bundle()).unwrap();
        let mut features = [0.0; FEATURE_DIM];
        assert_eq!(model.predict(&features).p_bot, 0.5);
        features[0] = 3.0;
        assert!(model.predict(&features).p_bot > 0.99);
        features[0] = f32::NAN;
        assert!(model.predict(&features).abstain);
        assert!(model.predict(&[]).abstain);
        let mut bad = bundle();
        bad.feature_dim += 1;
        assert!(Mlp::from_bundle(bad).is_err());
        let mut bad = bundle();
        bad.w1[0].pop();
        assert!(Mlp::from_bundle(bad).is_err());
        let mut bad = bundle();
        bad.schema_hash.clear();
        assert!(Mlp::from_bundle(bad).is_err());
    }
    #[test]
    fn artifact_decode_preserves_identity_and_rejects_malformed_json() {
        let model = Mlp::from_json(&serde_json::to_vec(&bundle()).unwrap()).unwrap();
        assert_eq!(model.schema_hash(), schema_hash());
        assert_eq!(model.bundle_version(), "test");
        assert!(Mlp::from_json(b"{}").is_err());
        assert!(Mlp::from_json(b"invalid").is_err());
        let path = std::env::temp_dir().join(format!(
            "humanymous-mlp-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, serde_json::to_vec(&bundle()).unwrap()).unwrap();
        let loaded = Mlp::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.bundle_version(), "test");
        let runtime = crate::inference::Runtime::default();
        runtime.set(Some(std::sync::Arc::new(loaded)));
        assert!(!runtime.score(&[0.0; FEATURE_DIM]).abstain);
        assert!(Mlp::load(&path).is_err());
    }
}
