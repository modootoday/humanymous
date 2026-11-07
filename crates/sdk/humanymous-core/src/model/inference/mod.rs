//! Validated model artifacts and the shared active/shadow inference boundary.
mod bundle;
mod mlp;
mod runtime;

pub use bundle::MlpBundle;
pub use mlp::Mlp;
pub use runtime::{
    AbstainPredictor, FeatureObserver, Prediction, Predictor, Runtime, ShadowObserver,
    ThresholdProvider,
};
