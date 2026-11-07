//! Actual shared authority/library + real WAL; synthetic evidence is test-only.
use humanymous_audit::{
    log::{Config, Log},
    record::Record,
    storage::{Wal, WalOptions},
};
use humanymous_authority::{
    Rejection,
    configuration::Configuration,
    decision::{ApplicationHttp, Authority, DurableAudit, Evidence},
    transport::Listener,
};
use humanymous_core::integrity::adapter::ValidatedRequest;
use humanymous_core::signals::report::SessionReport;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub struct Service {
    pub credential: String,
    pub durable: Arc<AtomicUsize>,
    pub fail: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Drop for Service {
    fn drop(&mut self) {
        self.task.abort();
    }
}
struct Metered {
    log: Log,
    count: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
}
impl DurableAudit for Metered {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        if self.fail.load(Ordering::Acquire) {
            return Err(std::io::Error::other("test-only audit dependency failure"));
        }
        self.log.append(record)?;
        self.count.fetch_add(1, Ordering::Release);
        Ok(())
    }
}
struct Fixture;
impl Evidence for Fixture {
    fn report(
        &mut self,
        request: &ValidatedRequest,
        raw: bool,
    ) -> Result<SessionReport, Rejection> {
        let path = request.request_target().1.split('?').next().unwrap_or("");
        if path == "/cancel" {
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        if path == "/timeout" {
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
        if path == "/production" {
            return ApplicationHttp.report(request, raw);
        }
        let file = match path {
            "/deny" => include_str!(
                "../../../sdk/humanymous-core/tests/scoring/fixtures/frozen/selenium-hr1.json"
            ),
            "/challenge" => {
                include_str!(
                    "../../../sdk/humanymous-core/tests/scoring/fixtures/frozen/no-client-hr10.json"
                )
            }
            _ => include_str!(
                "../../../sdk/humanymous-core/tests/scoring/fixtures/frozen/human-chrome-allow.json"
            ),
        };
        let value: serde_json::Value =
            serde_json::from_str(file).map_err(|_| Rejection::new(503, "fixture"))?;
        serde_json::from_value(value["report"].clone()).map_err(|_| Rejection::new(503, "fixture"))
    }
}
pub async fn start(
    base: &Path,
    paths: &[(&str, &str)],
) -> Result<Service, Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let directory = base.join("audit");
    std::fs::create_dir(&directory)?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    let required = serde_json::json!({"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":false});
    let routes: Vec<_> = paths
        .iter()
        .map(|(method, path)| {
            let mut needed = required.clone();
            if *path == "/capture" {
                needed["raw_tls"] = serde_json::json!(true);
            }
            serde_json::json!({"method":method,"path":path,"required":needed})
        })
        .collect();
    let mut key = [0; 32];
    let mut seed = [0; 32];
    getrandom::fill(&mut key)?;
    getrandom::fill(&mut seed)?;
    let config: Configuration = serde_json::from_value(serde_json::json!({
        "adapter":"nginx","endpoint":base.join("decision.sock"),"scope":{"installation":"nginx","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "peer":{"kind":"unix","uid":rustix::process::geteuid().as_raw()},"routes":routes,
        "audit":{"directory":directory,"node_id":"nginx-pilot","hmac_key_hex":hex::encode(key),"signing_seed_hex":hex::encode(seed),"allow_new":true,"trusted_head":null},
        "capacity":16,"replay_capacity":1024,"rate_per_second":1000,"timeout_ms":2000,"clock_skew_ms":1000
    }))?;
    config.validate().map_err(|_| "authority configuration")?;
    let log = Log::open(
        Wal::open(&directory, WalOptions::default())?,
        Config {
            node_id: "nginx-pilot".into(),
            hmac_key: key.to_vec(),
            signing_seed: seed,
            checkpoint_every: 1,
            witness: None,
        },
        None,
        true,
    )?;
    let durable = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicBool::new(false));
    let listener = Listener::bind(&config)?;
    let authority = Authority::new(
        config,
        Default::default(),
        Box::new(Fixture),
        Box::new(Metered {
            log,
            count: durable.clone(),
            fail: fail.clone(),
        }),
    )
    .map_err(|_| "authority startup")?;
    let credential = authority.credential().to_owned();
    let task = tokio::spawn(listener.serve(authority, 16, 1000, 2000));
    Ok(Service {
        credential,
        durable,
        fail,
        task,
    })
}
