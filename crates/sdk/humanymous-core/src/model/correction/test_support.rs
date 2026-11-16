//! Shared test-only helpers that replay the Go math/rand streams used by the
//! legacy tests through the existing pinned offline generator.
use crate::behavior::features::{FEATURE_DIM, FeatureVector};
use crate::model::training::rng::{GoRng, MAX_DRAWS};

/// A seeded Go-compatible stream with the largest supported draw budget.
pub(super) fn go_rng(seed: i64) -> GoRng {
    GoRng::new(seed, MAX_DRAWS).expect("valid draw budget")
}

/// Go `Rand.Float32`: narrow `Float64` and retry when it rounds to one.
pub(super) fn float32(rng: &mut GoRng) -> f32 {
    loop {
        let value = rng.float64().expect("draw budget") as f32;
        if value != 1.0 {
            return value;
        }
    }
}

/// Legacy `fvFrom`: mild noise on every feature, then a shiftable feature 0.
pub(super) fn feature_vector(rng: &mut GoRng, center: f64) -> FeatureVector {
    let mut vector = [0.0_f32; FEATURE_DIM];
    for value in &mut vector {
        *value = (rng.norm_float64().expect("draw budget") * 0.1) as f32;
    }
    vector[0] = (center + rng.norm_float64().expect("draw budget") * 0.3) as f32;
    vector
}
