//! Real authority lifecycle with test-only frozen evidence and audit reuse.
use super::{
    authority_fixture::{Observations, SyntheticEvidence, audit_log},
    wsgi_host::{Directory, configuration},
};
use humanymous_authority::{
    configuration::Configuration,
    decision::{Authority, Evidence},
    transport::Listener,
};
use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

pub(super) struct Gate {
    pub entered: Sender<()>,
    pub release: Receiver<()>,
}
struct GatedEvidence {
    evidence: SyntheticEvidence,
    gate: Option<Gate>,
}
impl Evidence for GatedEvidence {
    fn report(
        &mut self,
        request: &humanymous_integrity::adapter::ValidatedRequest,
        raw: bool,
    ) -> Result<humanymous_signals::report::SessionReport, humanymous_authority::Rejection> {
        if let Some(gate) = self.gate.take() {
            gate.entered
                .send(())
                .expect("request entered actual authority");
            gate.release
                .recv_timeout(Duration::from_secs(2))
                .expect("owned test decision release deadline");
        }
        self.evidence.report(request, raw)
    }
}
pub(super) struct Running {
    pub endpoint: String,
    pub credential: String,
    pub observed: Observations,
    runtime: tokio::runtime::Runtime,
    service: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Running {
    pub fn start(directory: &Directory, node: &str, gate: Option<Gate>) -> Self {
        let value = configuration(directory, node);
        let config: Configuration = serde_json::from_value(value.clone()).unwrap();
        let observed = Default::default();
        let authority = Authority::new(
            config,
            Default::default(),
            Box::new(GatedEvidence {
                evidence: SyntheticEvidence(std::sync::Arc::clone(&observed)),
                gate,
            }),
            Box::new(audit_log(&directory.0, node, true)),
        )
        .unwrap();
        let credential = authority.credential().to_owned();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let config: Configuration = serde_json::from_value(value.clone()).unwrap();
        let listener = {
            let _entered = runtime.enter();
            Listener::bind(&config).unwrap()
        };
        let service = runtime.spawn(async move { listener.serve(authority, 8, 100, 2000).await });
        Self {
            endpoint: value["endpoint"].as_str().unwrap().into(),
            credential,
            observed,
            runtime,
            service,
        }
    }
    pub fn finish(
        self,
        directory: &Directory,
        node: &str,
    ) -> Vec<humanymous_audit::record::Record> {
        self.service.abort();
        let joined = self.runtime.block_on(self.service);
        assert!(
            joined.is_err_and(|error| error.is_cancelled()),
            "owned authority listener must join cancellation"
        );
        self.runtime.shutdown_timeout(Duration::from_secs(2));
        audit_log(&directory.0, node, false).records().unwrap()
    }
}
