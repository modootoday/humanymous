use humanymous_behavior::features::FEATURE_DIM;
use serde::{Deserialize, Serialize};

/// Serialized standardization, one hidden layer and Platt calibration parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MlpBundle {
    /// Artifact version used for auditing.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::null_default")]
    pub version: String,
    /// Feature-schema compatibility digest.
    #[serde(
        rename = "schemaHash",
        deserialize_with = "humanymous_signals::wire::decode::null_default"
    )]
    pub schema_hash: String,
    /// Input width.
    #[serde(
        rename = "featureDim",
        deserialize_with = "humanymous_signals::wire::decode::null_default"
    )]
    pub feature_dim: i64,
    /// Hidden-layer width.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::null_default")]
    pub hidden: i64,
    /// Input standardization means.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::vec_default")]
    pub mean: Vec<f32>,
    /// Input standardization deviations; zero becomes one at inference.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::vec_default")]
    pub std: Vec<f32>,
    /// Hidden weights, row major.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::matrix_default")]
    pub w1: Vec<Vec<f32>>,
    /// Hidden biases.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::vec_default")]
    pub b1: Vec<f32>,
    /// Output weights.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::vec_default")]
    pub w2: Vec<f32>,
    /// Output bias.
    #[serde(deserialize_with = "humanymous_signals::wire::decode::null_default")]
    pub b2: f32,
    /// Platt logit multiplier.
    #[serde(
        rename = "calA",
        deserialize_with = "humanymous_signals::wire::decode::null_default"
    )]
    pub cal_a: f32,
    /// Platt logit offset.
    #[serde(
        rename = "calB",
        deserialize_with = "humanymous_signals::wire::decode::null_default"
    )]
    pub cal_b: f32,
}

impl MlpBundle {
    /// Reject incompatible dimensions, malformed shapes and missing schema identity.
    pub fn validate(&self) -> Result<(), String> {
        if self.feature_dim != FEATURE_DIM as i64 {
            return Err(format!(
                "mlserve: bundle featureDim {} != engine {}",
                self.feature_dim, FEATURE_DIM
            ));
        }
        if self.hidden <= 0
            || self.w1.len() != self.hidden as usize
            || self.b1.len() != self.hidden as usize
            || self.w2.len() != self.hidden as usize
        {
            return Err("mlserve: bundle hidden-layer shape invalid".into());
        }
        if self.mean.len() != FEATURE_DIM || self.std.len() != FEATURE_DIM {
            return Err("mlserve: bundle standardization shape invalid".into());
        }
        for (index, row) in self.w1.iter().enumerate() {
            if row.len() != FEATURE_DIM {
                return Err(format!(
                    "mlserve: bundle W1 row {index} width {} != {FEATURE_DIM}",
                    row.len()
                ));
            }
        }
        if self.schema_hash.is_empty() {
            return Err("mlserve: bundle missing schemaHash".into());
        }
        Ok(())
    }
}
