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
    os::unix::fs::MetadataExt,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

// Exact owned, freshly created WAL baseline. Never reopen empty history as new.
#[derive(Debug, PartialEq)]
struct AuditFile {
    name: String,
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
    bytes: Vec<u8>,
}
#[derive(Debug, PartialEq)]
struct EmptyAudit(Vec<AuditFile>);
impl EmptyAudit {
    fn capture(directory: &Directory) -> Self {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&directory.0).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            if name == ".audit-writer.lock"
                || name == "checkpoints.jsonl"
                || name.starts_with("audit-")
            {
                let metadata = std::fs::symlink_metadata(entry.path()).unwrap();
                assert!(metadata.file_type().is_file());
                files.push(AuditFile {
                    name,
                    device: metadata.dev(),
                    inode: metadata.ino(),
                    owner: metadata.uid(),
                    mode: metadata.mode(),
                    bytes: std::fs::read(entry.path()).unwrap(),
                });
            }
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));
        Self(files)
    }
}

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
    empty_audit: EmptyAudit,
    runtime: tokio::runtime::Runtime,
    service: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Running {
    pub fn start(directory: &Directory, node: &str, gate: Option<Gate>) -> Self {
        let value = configuration(directory, node);
        let config: Configuration = serde_json::from_value(value.clone()).unwrap();
        let observed = Default::default();
        let audit = audit_log(&directory.0, node, true);
        let empty_audit = EmptyAudit::capture(directory);
        assert_eq!(empty_audit.0.len(), 2);
        assert_eq!(empty_audit.0[0].name, ".audit-writer.lock");
        assert_eq!(empty_audit.0[1].name, "audit-0000000001.log");
        assert!(empty_audit.0.iter().all(|file| file.bytes.is_empty()));
        let authority = Authority::new(
            config,
            Default::default(),
            Box::new(GatedEvidence {
                evidence: SyntheticEvidence(std::sync::Arc::clone(&observed)),
                gate,
            }),
            Box::new(audit),
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
            empty_audit,
            runtime,
            service,
        }
    }
    pub fn finish(
        self,
        directory: &Directory,
        node: &str,
    ) -> Vec<humanymous_audit::record::Record> {
        self.shutdown();
        audit_log(&directory.0, node, false).records().unwrap()
    }
    pub fn finish_empty(self, directory: &Directory) {
        let before = EmptyAudit::capture(directory);
        assert_eq!(
            before, self.empty_audit,
            "owned empty WAL must remain unchanged"
        );
        self.shutdown();
        assert_eq!(
            EmptyAudit::capture(directory),
            before,
            "shutdown must preserve exact empty WAL files and identities"
        );
    }
    fn shutdown(self) {
        self.service.abort();
        let joined = self.runtime.block_on(self.service);
        assert!(
            joined.is_err_and(|error| error.is_cancelled()),
            "owned authority listener must join cancellation"
        );
        self.runtime.shutdown_timeout(Duration::from_secs(2));
    }
}
