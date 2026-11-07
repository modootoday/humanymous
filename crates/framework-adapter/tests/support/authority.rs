//! Shared host-test evidence and audit fixture; never part of a native library.
use humanymous_runtime::audit::{
    log::{Config, Log},
    storage::{Wal, WalOptions},
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub struct Observed {
    pub headers: Vec<[String; 2]>,
    pub mode: String,
}
pub type Observations = Arc<Mutex<Vec<Observed>>>;
pub struct SyntheticEvidence(pub Observations);
impl humanymous_runtime::authority::decision::Evidence for SyntheticEvidence {
    fn report(
        &mut self,
        request: &humanymous_core::integrity::adapter::ValidatedRequest,
        _: bool,
    ) -> Result<humanymous_core::signals::report::SessionReport, humanymous_runtime::authority::Rejection>
    {
        self.0.lock().unwrap().push(Observed {
            headers: request.headers().to_vec(),
            mode: request.header_mode().to_owned(),
        });
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../sdk/humanymous-core/tests/scoring/fixtures/frozen/human-chrome-allow.json"
        ))
        .unwrap();
        Ok(serde_json::from_value(fixture["report"].clone()).unwrap())
    }
}

pub fn audit_log(directory: &Path, node_id: &str, create: bool) -> Log {
    Log::open(
        Wal::open(directory, WalOptions::default()).unwrap(),
        Config {
            node_id: node_id.into(),
            hmac_key: vec![0xaa; 32],
            signing_seed: [0xbb; 32],
            checkpoint_every: 1,
            witness: None,
        },
        None,
        create,
    )
    .unwrap()
}
