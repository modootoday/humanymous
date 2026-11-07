//! Existing artifact schema and float32 inference results compared with Go MLPs.
use humanymous_core::model::inference::{Mlp, MlpBundle, Predictor};
use serde::Deserialize;

#[derive(Deserialize)]
struct Model {
    bundle: MlpBundle,
    vectors: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    features: Vec<f32>,
    expected: Expected,
}
#[derive(Deserialize)]
struct Expected {
    #[serde(rename = "PBot")]
    p_bot: f32,
    #[serde(rename = "Abstain")]
    abstain: bool,
}

#[test]
fn existing_models_and_forward_values_match_go() {
    let models: Vec<Model> = serde_json::from_str(include_str!("fixtures/inference.json")).unwrap();
    assert_eq!(models.len(), 5);
    for model in models {
        let predictor = Mlp::from_bundle(model.bundle).unwrap();
        assert_eq!(model.vectors.len(), 68);
        for case in model.vectors {
            let actual = predictor.predict(&case.features);
            assert_eq!(actual.abstain, case.expected.abstain);
            assert_eq!(actual.p_bot, case.expected.p_bot);
        }
    }
}
