use humanymous_behavior::features::schema_hash;
use std::sync::{Arc, RwLock};

/// Model probability, or an explicit abstention when no usable model evidence exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prediction {
    /// Calibrated automation probability.
    pub p_bot: f32,
    /// No usable prediction; contributes no model observation.
    pub abstain: bool,
}
impl Prediction {
    /// Neutral no-model result.
    pub fn abstained() -> Self {
        Self {
            p_bot: 0.0,
            abstain: true,
        }
    }
}

/// Immutable, concurrent inference implementation. Malformed vectors must abstain.
pub trait Predictor: Send + Sync {
    /// Infer from a potentially malformed feature slice.
    fn predict(&self, features: &[f32]) -> Prediction;
    /// Feature schema the predictor consumes.
    fn schema_hash(&self) -> &str;
    /// Artifact identity for auditing.
    fn bundle_version(&self) -> &str;
}

/// Default predictor: no evidence and no executable model.
pub struct AbstainPredictor;
impl Predictor for AbstainPredictor {
    fn predict(&self, _features: &[f32]) -> Prediction {
        Prediction::abstained()
    }
    fn schema_hash(&self) -> &str {
        ""
    }
    fn bundle_version(&self) -> &str {
        "none"
    }
}

/// Optional calibrated annotation threshold; never grants verdict authority.
pub trait ThresholdProvider: Send + Sync {
    /// Current model annotation cut-point.
    fn fire_threshold(&self) -> f32;
}
/// Nonblocking covariate monitor for schema-compatible inference inputs.
pub trait FeatureObserver: Send + Sync {
    /// Observe features before active prediction.
    fn observe_features(&self, features: &[f32]);
}
/// Comparator that receives active and candidate outputs without serving the candidate.
pub trait ShadowObserver: Send + Sync {
    /// Observe one active/candidate pair.
    fn observe_shadow(&self, active: Prediction, shadow: Prediction);
}

#[derive(Clone)]
struct State {
    active: Arc<dyn Predictor>,
    shadow: Option<Arc<dyn Predictor>>,
    threshold: Option<Arc<dyn ThresholdProvider>>,
    features: Option<Arc<dyn FeatureObserver>>,
    shadow_observer: Option<Arc<dyn ShadowObserver>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            active: Arc::new(AbstainPredictor),
            shadow: None,
            threshold: None,
            features: None,
            shadow_observer: None,
        }
    }
}

/// Shared hot-swap runtime. A request snapshots references before calling user
/// predictors/observers, so callbacks never run under the configuration lock.
#[derive(Clone, Default)]
pub struct Runtime {
    state: Arc<RwLock<State>>,
}

impl Runtime {
    /// Install an active model, or reset to abstention.
    pub fn set(&self, predictor: Option<Arc<dyn Predictor>>) {
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .active = predictor.unwrap_or_else(|| Arc::new(AbstainPredictor));
    }
    /// Current active predictor, never absent.
    pub fn current(&self) -> Arc<dyn Predictor> {
        self.state
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .active
            .clone()
    }
    /// Install or clear the candidate predictor.
    pub fn set_shadow(&self, predictor: Option<Arc<dyn Predictor>>) {
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .shadow = predictor;
    }
    /// Install or reset the dynamic model annotation cut-point.
    pub fn set_threshold_provider(&self, provider: Option<Arc<dyn ThresholdProvider>>) {
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .threshold = provider;
    }
    /// Install or detach covariate observation.
    pub fn set_feature_observer(&self, observer: Option<Arc<dyn FeatureObserver>>) {
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .features = observer;
    }
    /// Install or detach the candidate comparator.
    pub fn set_shadow_observer(&self, observer: Option<Arc<dyn ShadowObserver>>) {
        self.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .shadow_observer = observer;
    }
    /// Annotation cut-point; the default is 0.5.
    pub fn fire_threshold(&self) -> f32 {
        let provider = self
            .state
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .threshold
            .clone();
        provider
            .map(|provider| provider.fire_threshold())
            .unwrap_or(0.5)
    }
    /// Predict only under a matching feature schema, observe a compatible candidate,
    /// and always return the active result.
    pub fn score(&self, features: &[f32]) -> Prediction {
        let state = self
            .state
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let expected = schema_hash();
        if state.active.schema_hash().is_empty() || state.active.schema_hash() != expected {
            return Prediction::abstained();
        }
        if let Some(observer) = state.features {
            observer.observe_features(features);
        }
        let active = state.active.predict(features);
        if let Some(shadow) = state.shadow
            && shadow.schema_hash() == expected
            && let Some(observer) = state.shadow_observer
        {
            observer.observe_shadow(active, shadow.predict(features));
        }
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fixed {
        probability: f32,
        schema: String,
    }
    impl Predictor for Fixed {
        fn predict(&self, _: &[f32]) -> Prediction {
            Prediction {
                p_bot: self.probability,
                abstain: false,
            }
        }
        fn schema_hash(&self) -> &str {
            &self.schema
        }
        fn bundle_version(&self) -> &str {
            "test"
        }
    }
    #[derive(Default)]
    struct Capture(Mutex<Vec<(Prediction, Prediction)>>);
    impl ShadowObserver for Capture {
        fn observe_shadow(&self, active: Prediction, shadow: Prediction) {
            self.0.lock().unwrap().push((active, shadow));
        }
    }
    #[test]
    fn no_model_and_stale_schema_abstain() {
        let runtime = Runtime::default();
        assert!(runtime.score(&[]).abstain);
        runtime.set(Some(Arc::new(Fixed {
            probability: 0.99,
            schema: "stale".into(),
        })));
        assert!(runtime.score(&[]).abstain);
        runtime.set(Some(Arc::new(Fixed {
            probability: 0.8,
            schema: schema_hash(),
        })));
        assert_eq!(runtime.score(&[]).p_bot, 0.8);
        runtime.set(None);
        assert!(runtime.score(&[]).abstain);
    }
    #[test]
    fn candidate_is_observed_but_never_served_and_stale_candidates_are_skipped() {
        let runtime = Runtime::default();
        let capture = Arc::new(Capture::default());
        runtime.set(Some(Arc::new(Fixed {
            probability: 0.3,
            schema: schema_hash(),
        })));
        runtime.set_shadow(Some(Arc::new(Fixed {
            probability: 0.9,
            schema: schema_hash(),
        })));
        runtime.set_shadow_observer(Some(capture.clone()));
        assert_eq!(runtime.score(&[]).p_bot, 0.3);
        assert_eq!(capture.0.lock().unwrap().len(), 1);
        runtime.set_shadow(Some(Arc::new(Fixed {
            probability: 0.9,
            schema: "stale".into(),
        })));
        assert_eq!(runtime.score(&[]).p_bot, 0.3);
        assert_eq!(capture.0.lock().unwrap().len(), 1);
    }
    #[test]
    fn feature_tap_runs_only_for_compatible_active_models_and_thresholds_reset() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        #[derive(Default)]
        struct Features(AtomicUsize);
        impl FeatureObserver for Features {
            fn observe_features(&self, _: &[f32]) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        struct Threshold;
        impl ThresholdProvider for Threshold {
            fn fire_threshold(&self) -> f32 {
                0.75
            }
        }
        let runtime = Runtime::default();
        let capture = Arc::new(Features::default());
        runtime.set_feature_observer(Some(capture.clone()));
        runtime.score(&[]);
        assert_eq!(capture.0.load(Ordering::SeqCst), 0);
        runtime.set(Some(Arc::new(Fixed {
            probability: 0.8,
            schema: schema_hash(),
        })));
        runtime.score(&[]);
        assert_eq!(capture.0.load(Ordering::SeqCst), 1);
        runtime.set_feature_observer(None);
        runtime.score(&[]);
        assert_eq!(capture.0.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.fire_threshold(), 0.5);
        runtime.set_threshold_provider(Some(Arc::new(Threshold)));
        assert_eq!(runtime.fire_threshold(), 0.75);
        runtime.set_threshold_provider(None);
        assert_eq!(runtime.fire_threshold(), 0.5);
    }
}
