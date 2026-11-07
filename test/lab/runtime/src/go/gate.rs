//! Fresh per-case Gate with private independent witness and origin contact relay.
use super::{Result, relay};
use crate::installation::witnessed::{self, WitnessedInstallation};
use humanymous_audit::{
    checkpoint::service,
    storage::{RecoveryPolicy, TailRecovery, Wal, plan_witness_recovery},
};
use humanymous_runtime::service::{ServiceHost, ShutdownRequest};
use serde_json::json;
use std::{
    io::{self, BufReader, Read},
    net::SocketAddr,
    path::Path,
    time::Duration,
};
use tokio::{net::TcpListener, sync::watch};

pub fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 2 {
        return Err("owned backend relay tuple and Go case required".into());
    }
    let peer: SocketAddr = args[0].parse()?;
    if !matches!(peer.ip(),std::net::IpAddr::V4(ip)if ip.is_private()) || peer.port() != 18082 {
        return Err("owned Go backend tuple required".into());
    }
    let case: humanymous::adapters::go::Case = serde_json::from_value(json!(args[1]))?;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    executor.block_on(async move {
        let contact_listener=TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0)).await?;
        let address=contact_listener.local_addr()?;
        let WitnessedInstallation{installation,external,writer_public,witness_public}=witnessed::fresh(BufReader::new(io::stdin()),|installation|{
            installation.config["origin"]=json!(address);
            installation.config=super::configuration_value(installation.config.clone(),case==humanymous::adapters::go::Case::ForgedProvenance)?;
            Ok(())
        })?;
        let config=super::deployment(installation.config.clone())?;
        let host=ServiceHost::bind_with_witness(config,external).await?;
        let runtime_address=host.readiness().listen;
        let ingress=TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED,8080)).await?;
        let contacts=relay::Contacts::default();let ingress_contacts=relay::Contacts::default();
        let(stop,receiver)=watch::channel(false);let(shutdown,controller)=watch::channel(None);
        let origin=tokio::spawn(relay::serve(contact_listener,peer,contacts.clone(),receiver.clone()));
        let frontend=tokio::spawn(relay::serve(ingress,runtime_address,ingress_contacts.clone(),receiver));
        let runtime=tokio::spawn(host.run(controller));
        let setup_config=super::deployment(installation.config.clone())?;
        tokio::task::spawn_blocking(move||{
            // Establish one real witnessed audit record for every fresh case,
            // including parser/kernel negatives. This setup never calls origin.
            let issued=crate::client::issue(runtime_address);
            let mut report=crate::client::request("/_humanymous/collect",crate::client::report(&issued));
            crate::client::sign(&setup_config,&mut report,&issued,1);
            crate::client::expect(runtime_address,&report,200);
        }).await?;
        if contacts.count()!=0{return Err("Go setup unexpectedly contacted origin".into());}
        let mut sanitized=installation.config.clone();sanitized["audit"]["keys"]["passphrase"]=json!("<private>");
        service::write_frame(&mut io::stdout(),&json!({"role":"go_gate_ready","case":case,"runtime":runtime_address,"contact_witness":address,"backend":peer,"frontend":"0.0.0.0:8080","profile":host_profile(),"configuration":sanitized,"configuration_sha256":configuration_digest(&installation.config)?}))?;
        let until=tokio::time::Instant::now()+Duration::from_secs(120);
        loop{
            let bytes=serde_json::to_vec(&json!({"schema":"humanymous.go-contact-snapshot/1","case":case,"origin":contacts.snapshot(),"ingress":ingress_contacts.snapshot()}))?;
            std::fs::write("/tmp/go-contacts.pending",bytes)?;
            std::fs::rename("/tmp/go-contacts.pending","/tmp/go-contacts.json")?;
            if Path::new("/tmp/browser-pilot-stop").exists(){break;}
            if tokio::time::Instant::now()>=until{return Err("Go case supervisor deadline".into());}
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        stop.send(true).map_err(|_|io::Error::other("Go relay controller closed"))?;
        shutdown.send(Some(ShutdownRequest::new(5000)?)).map_err(|_|io::Error::other("Go runtime controller closed"))?;
        frontend.await??;origin.await??;
        let exit=tokio::time::timeout(Duration::from_secs(10),runtime).await???;
        let mut pin=Vec::new();std::fs::File::open("/tmp/browser-pilot-stop")?.take(16385).read_to_end(&mut pin)?;
        let pin=humanymous_platform::testing::json::decode_strict_json(&pin,16384)?;
        let external=serde_json::from_value(pin)?;
        let wal=Wal::open(Path::new("/writer/wal"),Default::default())?;
        let plan=plan_witness_recovery(wal.records()?,wal.checkpoints()?,RecoveryPolicy{node_id:"isolated-node",hmac_key:&installation.hmac,writer_public:&writer_public,witness_public:Some(&witness_public),trusted_head:Some(&external),allow_new:false},TailRecovery::RequireCovered).map_err(io::Error::other)?;
        if plan.needs_checkpoint_append(){return Err("normal Go case requires unexpected recovery".into());}
        service::write_frame(&mut io::stdout(),&json!({"role":"go_gate_result","case":case,"stopped":exit.stopped,"origin":contacts.snapshot(),"ingress":ingress_contacts.snapshot(),"witness_coverage":plan.coverage(),"checkpoint_repair_performed":false,"full_product_qualified":false}))?;
        Ok(())
    })
}
fn host_profile() -> &'static str {
    "synthetic-trusted-session-http-conformance"
}
fn configuration_digest(value: &serde_json::Value) -> Result<String> {
    use sha2::{Digest, Sha256};
    // Passphrase and ephemeral keys are excluded; the parent independently
    // hashes the complete sanitized per-case bundle before qualification.
    let mut value = value.clone();
    value["audit"]["keys"]["passphrase"] = json!("<private>");
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&value)?)))
}
pub fn count() -> Result<()> {
    let bytes = std::fs::read("/tmp/go-contacts.json")?;
    let value = humanymous_platform::testing::json::decode_strict_json(&bytes, 16384)?;
    println!("{value}");
    Ok(())
}
