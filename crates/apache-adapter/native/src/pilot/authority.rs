//! Real shared authority and WAL; synthetic frozen evidence is component-only.
use humanymous_audit::{
    log::{Config, Log},
    storage::{Wal, WalOptions},
};
use humanymous_authority::{
    Rejection,
    configuration::Configuration,
    decision::{ApplicationHttp, Authority, Evidence},
    transport::Listener,
};
use humanymous_integrity::adapter::ValidatedRequest;
use humanymous_signals::report::SessionReport;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub struct Service {
    pub credential: String,
    pub pending: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Service {
    pub async fn stop(&mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.task.abort();
    }
}
struct Frozen {
    pending: Arc<AtomicUsize>,
}
impl Evidence for Frozen {
    fn report(
        &mut self,
        request: &ValidatedRequest,
        raw: bool,
    ) -> Result<SessionReport, Rejection> {
        let path = request.request_target().1.split('?').next().unwrap_or("");
        if path == "/production" {
            return ApplicationHttp.report(request, raw);
        }
        if path == "/slow" {
            std::thread::sleep(std::time::Duration::from_millis(1200));
        }
        if path == "/cancel" {
            self.pending.fetch_add(1, Ordering::Release);
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let fixture = match path {
            "/deny" => include_str!("../../../../scoring/tests/fixtures/frozen/selenium-hr1.json"),
            "/challenge" => {
                include_str!("../../../../scoring/tests/fixtures/frozen/no-client-hr10.json")
            }
            _ => include_str!("../../../../scoring/tests/fixtures/frozen/human-chrome-allow.json"),
        };
        let value: serde_json::Value = serde_json::from_str(fixture)
            .map_err(|_| Rejection::new(503, "fixture_unavailable"))?;
        serde_json::from_value(value["report"].clone())
            .map_err(|_| Rejection::new(503, "fixture_unavailable"))
    }
}
pub async fn start(base: &Path) -> Result<Service, super::Error> {
    use std::os::unix::fs::PermissionsExt;
    let directory = base.join("audit");
    std::fs::create_dir(&directory)?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    let mut key = [0u8; 32];
    let mut seed = [0u8; 32];
    getrandom::fill(&mut key)?;
    getrandom::fill(&mut seed)?;
    let routes:Vec<_>=super::ROUTES.iter().map(|path|serde_json::json!({"method":"POST","path":path,
        "required":{"raw_tls":*path=="/capture","original_h2":false,"raw_header_order":false,"collector":false,"rit":false}})).collect();
    let config: Configuration = serde_json::from_value(serde_json::json!({"adapter":"apache",
        "endpoint":base.join("decision.sock"),"scope":{"installation":"apache","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "peer":{"kind":"unix","uid":rustix::process::geteuid().as_raw()},"routes":routes,
        "audit":{"directory":directory,"node_id":"apache-pilot","hmac_key_hex":hex::encode(key),"signing_seed_hex":hex::encode(seed),"allow_new":true,"trusted_head":null},
        "capacity":16,"replay_capacity":1024,"rate_per_second":1000,"timeout_ms":1800,"clock_skew_ms":1000}))?;
    config.validate().map_err(|_| "authority configuration")?;
    let log = Log::open(
        Wal::open(&directory, WalOptions::default())?,
        Config {
            node_id: "apache-pilot".into(),
            hmac_key: key.to_vec(),
            signing_seed: seed,
            checkpoint_every: 1,
            witness: None,
        },
        None,
        true,
    )?;
    let listener = Listener::bind(&config)?;
    let pending = Arc::new(AtomicUsize::new(0));
    let authority = Authority::new(
        config,
        Default::default(),
        Box::new(Frozen {
            pending: pending.clone(),
        }),
        Box::new(log),
    )
    .map_err(|_| "authority startup")?;
    let credential = authority.credential().to_owned();
    let task = tokio::spawn(listener.serve(authority, 16, 1000, 1800));
    Ok(Service {
        credential,
        task,
        pending,
    })
}
