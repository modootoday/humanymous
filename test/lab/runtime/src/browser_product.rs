//! Two-namespace product actors. Private witness frames never enter HTTP/logs.
use crate::browser_proxy::{self, ProxyConfig, Receipts};
use humanymous_audit::checkpoint::service;
use humanymous_runtime::service::{ServiceHost, ShutdownRequest};
use serde_json::json;
use std::{
    io::{self, BufReader, Read, Write},
    net::SocketAddr,
    path::Path,
    time::Duration,
};
use tokio::{net::TcpListener, sync::watch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct BrowserConfig {
    gate: SocketAddr,
    runtime: SocketAddr,
    backend: SocketAddr,
    dns_canary: std::net::Ipv4Addr,
    dns_name: String,
}

fn hardening() -> Result<serde_json::Value> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .map(str::trim)
    };
    let uid = field("Uid:").ok_or("actual UID absent")?;
    if !uid.split_whitespace().all(|id| id == "65532")
        || field("CapEff:") != Some("0000000000000000")
        || field("NoNewPrivs:") != Some("1")
    {
        return Err("actual actor privilege boundary".into());
    }
    if Path::new("/var/run/docker.sock").exists() {
        return Err("Docker socket exposed".into());
    }
    let error = std::fs::write("/.humanymous-browser-write-probe", b"probe")
        .expect_err("actor root must be read-only");
    if error.raw_os_error() != Some(30) {
        eprintln!(
            "HM-LAB-ACTOR-STEP {}",
            if error.raw_os_error() == Some(13) {
                "actor_root_probe_eacces"
            } else {
                "actor_root_probe_unknown"
            }
        );
        return Err("actual root EROFS missing".into());
    }
    Ok(
        json!({"uid":uid,"cap_eff":"0","no_new_privs":true,"root_erofs":true,"docker_socket_absent":true,"net_namespace":std::fs::read_link("/proc/self/ns/net")?.to_string_lossy()}),
    )
}

pub fn gate() -> Result<()> {
    eprintln!("HM-LAB-ACTOR-STEP actor_hardening");
    let hardening = hardening()?;
    let input = BufReader::new(io::stdin());
    // One fresh installation per product bundle; credentials are process-private.
    let crate::installation::witnessed::WitnessedInstallation {
        installation,
        external,
        writer_public: public,
        witness_public: witness,
    } = crate::installation::witnessed::fresh(input, |installation| {
        let fixture = browser_proxy::fixture();
        let config = &mut installation.config;
        config["scope"] = fixture["scope"].clone();
        config["browser"] = json!({"origin":fixture["browser_origin"]});
        let route = |path: &serde_json::Value, collector: bool| json!({"method":"POST","path":path,"rit_profile":"browser_auth_only","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":collector,"rit":true}});
        config["routes"] = json!([
            route(&fixture["routes"]["effect"], false),
            route(&fixture["routes"]["required_collector"], true)
        ]);
        Ok(())
    })?;
    let config = serde_json::from_value(installation.config.clone())?;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    executor.block_on(async move {
        let host=ServiceHost::bind_with_witness(config,external).await?;
        let listen:SocketAddr=host.readiness().listen;
        let actual_profile=host.readiness().profile.clone();
        let gate=TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED,8080)).await?;
        let receipts=Receipts::default();
        let (stop,receiver)=watch::channel(false);
        let (shutdown,controller)=watch::channel(None);
        let config=ProxyConfig{listen:gate.local_addr()?,upstream:listen,browser:false};
        let proxy=tokio::spawn(browser_proxy::serve(gate,config,receiver,receipts.clone()));
        let runtime=tokio::spawn(host.run(controller));
        service::write_frame(&mut io::stdout(),&json!({"role":"browser_gate_ready","runtime":listen,"backend":installation.origin.address,"frontend":"0.0.0.0:8080","profile":actual_profile,"hardening":hardening}))?;
        // Readiness and witness traffic share one audit-owned private protocol.
        // The host controls lifetime; no public shutdown endpoint exists.
        let deadline=tokio::time::Instant::now()+Duration::from_secs(150);
        while !Path::new("/tmp/browser-pilot-stop").exists() {
            if tokio::time::Instant::now()>=deadline {return Err("browser supervisor shutdown deadline".into());}
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        stop.send(true).map_err(|_|io::Error::other("proxy controller closed"))?;
        shutdown.send(Some(ShutdownRequest::new(5000)?)).map_err(|_|io::Error::other("runtime controller closed"))?;
        proxy.await??;
        let exit=tokio::time::timeout(Duration::from_secs(10),runtime).await???;
        let wal=humanymous_audit::storage::Wal::open(Path::new("/writer/wal"),Default::default())?;
        let mut pin=Vec::new();std::fs::File::open("/tmp/browser-pilot-stop")?.take(16385).read_to_end(&mut pin)?;
        if pin.len()>16384{return Err("external pin control bound".into());}
        let external:humanymous_audit::checkpoint::Checkpoint=serde_json::from_slice(&pin)?;
        let plan=humanymous_audit::storage::plan_witness_recovery(wal.records()?,wal.checkpoints()?,humanymous_audit::storage::RecoveryPolicy{node_id:"isolated-node",hmac_key:&installation.hmac,writer_public:&public,witness_public:Some(&witness),trusted_head:Some(&external),allow_new:false},humanymous_audit::storage::TailRecovery::RequireCovered).map_err(io::Error::other)?;
        if plan.needs_checkpoint_append(){return Err("normal browser shutdown unexpectedly requires checkpoint repair".into());}
        let history=plan.history();
        let records:Vec<_>=history.records().iter().map(|record|json!({"seq":record.seq,"event_type":record.event_type,"verdict":record.verdict})).collect();
        service::write_frame(&mut io::stdout(),&json!({"role":"browser_gate_result","stopped":exit.stopped,"origin_effects":installation.origin.effects(),"proxy_hits":receipts.snapshot(),"audit_sequence":history.sequence(),"audit_records":records,"audit_checkpoints":history.checkpoints(),"offline_log_authenticated":true,"witness_coverage":plan.coverage(),"checkpoint_repair_performed":false,"full_product_qualified":false}))?;
        Ok::<(),Box<dyn std::error::Error>>(())
    })
}

pub fn stop() -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let input = std::env::args()
        .nth(2)
        .ok_or("independently pinned public head required")?;
    if input.len() > 16384 {
        return Err("external pin control bound".into());
    }
    let head: humanymous_audit::checkpoint::Checkpoint = serde_json::from_str(&input)?;
    let pending = "/tmp/browser-pilot-stop.pending";
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(pending)?;
    file.write_all(&serde_json::to_vec(&head)?)?;
    file.sync_all()?;
    drop(file);
    std::fs::hard_link(pending, "/tmp/browser-pilot-stop")?;
    std::fs::remove_file(pending)?;
    Ok(())
}

pub fn browser() -> Result<()> {
    let hardening = hardening()?;
    // Owned endpoint tuples carry no credentials. No -i avoids Docker EOF detach.
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 5 {
        return Err("owned browser endpoint/DNS control tuples required".into());
    }
    let config = BrowserConfig {
        gate: args[0].parse()?,
        runtime: args[1].parse()?,
        backend: args[2].parse()?,
        dns_canary: args[3].parse()?,
        dns_name: args[4].clone(),
    };
    if !matches!(config.gate.ip(),std::net::IpAddr::V4(ip) if ip.is_private())
        || config.gate.port() != 8080
        || !config.runtime.ip().is_loopback()
        || !config.backend.ip().is_loopback()
        || config.runtime.port() == 0
        || config.backend.port() == 0
    {
        return Err("owned gate tuple required".into());
    }
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    executor.block_on(async move {
        let fixture=browser_proxy::fixture();
        let dns=crate::browser_dns::DenyResolver::start()?;
        let dns_control=crate::browser_dns::probe(&dns,&config.dns_name,config.dns_canary).await?;
        let mut negatives=serde_json::Map::new();
        negatives.insert("external_dns_local_policy_refused".into(),json!(true));
        negatives.insert("external_dns_udp_no_route".into(),json!(true));
        let mut refusals=serde_json::Map::new();
        for (name,address) in [("runtime_loopback_unreachable",config.runtime),("backend_loopback_unreachable",config.backend),("runtime_bridge_port_unreachable",SocketAddr::new(config.gate.ip(),config.runtime.port())),("backend_bridge_port_unreachable",SocketAddr::new(config.gate.ip(),config.backend.port())),("metadata_egress_refused","169.254.169.254:80".parse()?)] {
            let refused=match tokio::time::timeout(Duration::from_millis(300),tokio::net::TcpStream::connect(address)).await {
                Ok(Err(error)) if matches!(error.kind(),io::ErrorKind::ConnectionRefused|io::ErrorKind::NetworkUnreachable|io::ErrorKind::HostUnreachable|io::ErrorKind::PermissionDenied)=>{refusals.insert(name.into(),json!({"kind":format!("{:?}",error.kind()),"os_error":error.raw_os_error()}));true},
                Ok(Err(error))=>{eprintln!("{}",json!({"control":name,"status":"unverified","error_kind":format!("{:?}",error.kind()),"os_error":error.raw_os_error()}));false},
                Ok(Ok(_))=>{eprintln!("{}",json!({"control":name,"status":"failed","reason":"unexpected connection"}));false},
                Err(_)=>{eprintln!("{}",json!({"control":name,"status":"unverified","reason":"timeout"}));false},
            };
            negatives.insert(name.into(),json!(refused));
            if !refused {return Err("namespace/egress boundary failed".into());}
        }
        let routes=std::fs::read_to_string("/proc/net/route")?;
        let no_default=!routes.lines().skip(1).any(|line|line.split_whitespace().nth(1)==Some("00000000"));
        negatives.insert("no_default_ipv4_route".into(),json!(no_default));
        if !no_default {return Err("external default route present".into());}
        let listener=TcpListener::bind(fixture["browser_proxy_listen"].as_str().ok_or("proxy fixture listen")?).await?;
        let receipts=Receipts::default();let (stop,receiver)=watch::channel(false);
        let proxy=tokio::spawn(browser_proxy::serve(listener,ProxyConfig{listen:"127.0.0.1:31112".parse()?,upstream:config.gate,browser:true},receiver,receipts.clone()));
        let expected_name=fixture["test_selection"]["exact_name"].as_str().ok_or("fixture selected test")?.to_owned();
        let declared_filtered=fixture["test_selection"]["component_tests_filtered"].as_u64().ok_or("fixture filtered count")? as usize;
        if fixture["test_selection"]["selected"]!=1 {return Err("one fixture-selected test required".into());}
        let result=tokio::task::spawn_blocking(move || {
            let mut listing=std::process::Command::new("wasm-bindgen-test-runner");
            listing.args(["/usr/local/lib/collector-product-tests.wasm","--list"]);
            let inventory=humanymous_test_support::process::capture_with_stderr(&mut listing,b"",Duration::from_secs(10),128*1024)?;
            if !inventory.status.success(){return Err(io::Error::other("WASM test inventory failed"));}
            let names:Vec<String>=String::from_utf8(inventory.stdout).map_err(io::Error::other)?.lines().filter_map(|line|line.trim().strip_suffix(": test").map(str::to_owned)).collect();
            if names.iter().filter(|name|**name==expected_name).count()!=1 || names.len()!=declared_filtered+1 || names.iter().collect::<std::collections::HashSet<_>>().len()!=names.len(){return Err(io::Error::other("exact declared product test inventory required"));}
            let name=expected_name;
            let mut command=std::process::Command::new("wasm-bindgen-test-runner");
            command.args(["/usr/local/lib/collector-product-tests.wasm","--exact",&name])
                .env("WASM_BINDGEN_TEST_ADDRESS","127.0.0.1:31111")
                .env("WASM_BINDGEN_TEST_WEBDRIVER_JSON","/usr/local/lib/browser-pilot-webdriver.json")
                .env_remove("NO_HEADLESS");
            let captured=humanymous_test_support::process::capture_with_stderr(&mut command,b"",Duration::from_secs(120),1024*1024)?;
            Ok::<_,io::Error>((captured,name,names))
        }).await?;
        let (result,name,inventory)=result?;
        let output=String::from_utf8(result.stdout)?;
        let passed=result.status.success() && crate::browser_result::completed(&output,&name,inventory.len()-1);
        for (name,request) in [("foreign_proxy_authority_refused",b"GET http://169.254.169.254/ HTTP/1.1\r\nHost: 169.254.169.254\r\n\r\n".as_slice()),("unknown_proxy_api_refused",b"POST http://127.0.0.1:31111/_humanymous/forbidden HTTP/1.1\r\nHost: 127.0.0.1:31111\r\nContent-Length: 0\r\n\r\n".as_slice())] {
            use tokio::io::{AsyncReadExt,AsyncWriteExt};
            let mut socket=tokio::net::TcpStream::connect("127.0.0.1:31112").await?;
            socket.write_all(request).await?;
            let mut response=Vec::new();
            tokio::time::timeout(Duration::from_secs(2),socket.take(4096).read_to_end(&mut response)).await??;
            let refused=response.starts_with(b"HTTP/1.1 503 ");
            negatives.insert(name.into(),json!(refused));
            if !refused {return Err("proxy negative control failed".into());}
        }
        stop.send(true).map_err(|_|io::Error::other("proxy stopped"))?;proxy.await??;
        let dns_policy=dns.finish()?;
        println!("{}",json!({"schema":"humanymous.browser-product-actor/1","status":if passed{"passed"}else{"failed"},"test_output":output,"test_stderr":String::from_utf8_lossy(&result.stderr),"test_inventory":inventory,"selected_test":name,"declared_selected":1,"declared_filtered":inventory.len()-1,"summary_verified":passed,"proxy_hits":receipts.snapshot(),"namespace_controls":negatives,"connection_refusals":refusals,"hardening":hardening,"dns_control":dns_control,"dns_policy":dns_policy,"full_product_qualified":false}));
        if !passed {return Err("actual browser runtime pilot failed".into());}
        Ok::<(),Box<dyn std::error::Error>>(())
    })
}
