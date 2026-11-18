//! Roadmap 2e Pass recovery: the shipped runtime binary in this network-none
//! container serves an `attest_floor` route plus the legacy Pass surface; a
//! Rust HTTP/1.1 client in the same container is challenged, obtains a Pass
//! instance, pays the proof of work, solves from the public scene, redeems the
//! step-up receipt and is admitted to the protected route. Every refusal is
//! asserted on the wire and on the owned origin's contact counter.
//!
//! The client submits no motor trace (keyboard lane); nothing here gates on or
//! fabricates human timing. Keyboard and assistive human runs stay
//! human-attested and are out of this automated case.
use crate::{
    client::{request, sign},
    installation::Installation,
    process,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_core::challenge::{pass, proof_of_work};
use humanymous_runtime::gateway::{
    configuration::Configuration,
    http::Request,
    legacy::{STEP_UP_COOKIE, STEP_UP_PATH, STEP_UP_RECEIPT_HEADER},
    token::RECEIPT_SKEW,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Report schema read by the lab runner.
pub const SCHEMA: &str = "humanymous.pass-recovery-lab/1";
/// In-image copy of `web/pass.html` (deployments/lab/images/product.Dockerfile).
pub const WEB_DIR: &str = crate::product_e2e::HUMAN_TEST_WEB_DIR;
/// The attestation-floor route.
pub const ATTESTED: &str = "/attested";
/// Recovery rows, in execution order.
pub const RECOVERY: &[&str] = &[
    "attested-route-challenged",
    "pass-instance-issued",
    "proof-of-work-accepted",
    "solve-accepted-with-receipt",
    "receipt-redeemed",
    "recovered-session-admitted",
    "positive-control-after-negatives",
];
/// Refusal rows.
pub const REFUSED: &[&str] = &[
    "missing-pow-refused",
    "missing-receipt-refused",
    "forged-receipt-refused",
    "wrong-session-receipt-refused",
    "lifted-proof-refused",
    "wrong-session-proof-refused",
    "scoring-challenge-not-overridden",
    "replayed-solve-refused",
    "expired-receipt-refused",
];
const USER_AGENT: &str = "Mozilla/5.0 Chrome/126.0 Safari/537.36";
/// Upper bound on the wait for the genuine receipt to expire.
const EXPIRY_BUDGET: Duration = Duration::from_secs(300);

/// The pilot's private gateway document: the installation's routes plus one
/// attested route, and only the Pass legacy surface.
pub fn configuration(mut base: Value, web_dir: &str) -> Value {
    if let Some(routes) = base["routes"].as_array_mut() {
        routes.push(json!({"method":"POST","path":ATTESTED,"attest_floor":true,
            "required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}));
    }
    base["legacy"] = json!({"web_dir":web_dir,"pass":true});
    base["session_capacity"] = json!(32);
    base
}

struct Reply {
    status: Option<u16>,
    cookies: Vec<String>,
    body: Value,
}

fn exchange(address: SocketAddr, bytes: &[u8]) -> Reply {
    let result = (|| -> Result<Vec<u8>> {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(bytes)?;
        let mut out = Vec::new();
        stream.take(1 << 20).read_to_end(&mut out)?;
        Ok(out)
    })();
    let Ok(bytes) = result else {
        return Reply {
            status: None,
            cookies: Vec::new(),
            body: Value::Null,
        };
    };
    let end = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(bytes.len());
    let head = String::from_utf8_lossy(&bytes[..end]);
    Reply {
        status: head.get(9..12).and_then(|code| code.parse().ok()),
        cookies: head
            .split("\r\n")
            .filter_map(|line| line.strip_prefix("Set-Cookie: "))
            .filter_map(|value| value.split(';').next())
            .map(str::to_owned)
            .collect(),
        body: serde_json::from_slice(bytes.get(end + 4..).unwrap_or_default())
            .unwrap_or(Value::Null),
    }
}

fn wire(method: &str, target: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut bytes = format!("{method} {target} HTTP/1.1\r\nHost: owned.test\r\nUser-Agent: {USER_AGENT}\r\nsec-ch-ua: Chromium\r\nAccept: */*\r\n").into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    if !body.is_empty() {
        bytes.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(body);
    bytes
}

fn row(id: &str, passed: bool, observed: Value) -> Value {
    json!({"id":id,"status":if passed{"passed"}else{"failed"},"observed":observed})
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Public expiry of a `sur|sid|exp` receipt; the MAC is never inspected.
fn receipt_expiry(receipt: &str) -> Option<u64> {
    let payload = URL_SAFE_NO_PAD.decode(receipt.split('.').next()?).ok()?;
    let text = String::from_utf8(payload).ok()?;
    let mut fields = text.split('|');
    (fields.next()? == "sur").then_some(())?;
    fields.nth(1)?.parse().ok()
}

/// Keyboard-lane solve: offsets from the public scene, no motor trace.
fn solve_body(issued: &Value) -> Result<Vec<u8>> {
    let scene: pass::Challenge = serde_json::from_value(issued["challenge"].clone())?;
    let offsets: Vec<i64> = scene
        .rows
        .iter()
        .map(|row| (scene.center as i64 - row.key_index as i64).rem_euclid(scene.n as i64))
        .collect();
    Ok(serde_json::to_vec(
        &json!({"bucket":issued["bucket"],"challengeNonce":issued["challengeNonce"],"offsets":offsets}),
    )?)
}

struct Pilot {
    address: SocketAddr,
    config: Configuration,
    installation: Installation,
}

impl Pilot {
    fn effects(&self) -> usize {
        self.installation.origin.effects()
    }

    fn pass_new(&self, cookie: Option<&str>) -> (Option<String>, Reply) {
        let headers: Vec<(&str, &str)> = cookie.map(|c| vec![("Cookie", c)]).unwrap_or_default();
        let reply = exchange(self.address, &wire("GET", "/api/pass/new", &headers, b""));
        let cookie = cookie.map(str::to_owned).or_else(|| {
            reply
                .cookies
                .iter()
                .find(|c| c.starts_with("hsid="))
                .cloned()
        });
        (cookie, reply)
    }

    fn post(&self, path: &str, cookie: &str, extra: &[(&str, &str)], body: &[u8]) -> Reply {
        let mut headers = vec![("Cookie", cookie)];
        headers.extend_from_slice(extra);
        exchange(self.address, &wire("POST", path, &headers, body))
    }

    fn pow(&self, cookie: &str, issued: &Value) -> Result<Reply> {
        let challenge = &issued["preflight"]["pow"];
        let nonce = proof_of_work::solve(
            challenge["seed"].as_str().ok_or("pow seed")?,
            i32::try_from(challenge["difficulty"].as_i64().ok_or("pow difficulty")?)?,
            1 << 26,
        )
        .ok_or("proof of work not found")?;
        let body = serde_json::to_vec(&json!({"bucket":challenge["bucket"],"nonce":nonce,
            "challengeNonce":issued["challengeNonce"]}))?;
        Ok(self.post("/api/pass/pow", cookie, &[], &body))
    }

    fn redeem(&self, cookie: &str, receipt: Option<&str>) -> Reply {
        let extra: Vec<(&str, &str)> = receipt
            .map(|r| vec![(STEP_UP_RECEIPT_HEADER, r)])
            .unwrap_or_default();
        self.post(STEP_UP_PATH, cookie, &extra, b"{}")
    }

    /// One fresh RIT session (collected unless `collect` is false) sending a
    /// signed attested request; returns the reply and the origin contact delta.
    fn attested(
        &self,
        cookie: Option<&str>,
        user_agent: &str,
        collect: bool,
    ) -> Result<(Reply, usize)> {
        let issued = exchange(
            self.address,
            &crate::client::wire(&request("/_humanymous/session", vec![])),
        );
        if issued.status != Some(200) {
            return Err("RIT session issue refused".into());
        }
        let issued = issued.body;
        if collect {
            let mut req = request("/_humanymous/collect", crate::client::report(&issued));
            sign(&self.config, &mut req, &issued, 1);
            if exchange(self.address, &crate::client::wire(&req)).status != Some(200) {
                return Err("collect refused".into());
            }
        }
        let mut req: Request = request(ATTESTED, b"effect".to_vec());
        for header in &mut req.headers {
            if header.0 == "User-Agent" {
                header.1 = user_agent.into();
            }
        }
        if let Some(cookie) = cookie {
            req.headers.push(("Cookie".into(), cookie.into()));
        }
        sign(&self.config, &mut req, &issued, if collect { 2 } else { 1 });
        let before = self.effects();
        let reply = exchange(self.address, &crate::client::wire(&req));
        Ok((reply, self.effects() - before))
    }
}

fn challenged(reply: &Reply, delta: usize) -> bool {
    reply.status == Some(403) && reply.body["verdict"] == "CHALLENGE" && delta == 0
}

fn admitted(reply: &Reply, delta: usize) -> bool {
    reply.status == Some(200) && delta == 1
}

fn observed(reply: &Reply, delta: usize) -> Value {
    json!({"status":reply.status,"verdict":reply.body["verdict"],"origin_delta":delta})
}

fn execute(pilot: &Pilot) -> Result<Vec<Value>> {
    let mut rows = Vec::new();

    // A collected session scores ALLOW; the attestation floor challenges it.
    let (reply, delta) = pilot.attested(None, USER_AGENT, true)?;
    rows.push(row(
        "attested-route-challenged",
        challenged(&reply, delta) && reply.body["step_up"] == "/pass",
        observed(&reply, delta),
    ));

    let (cookie, reply) = pilot.pass_new(None);
    let cookie = cookie.ok_or("Pass session cookie absent")?;
    let issued = reply.body;
    rows.push(row(
        "pass-instance-issued",
        reply.status == Some(200)
            && issued["ok"] == true
            && issued["preflight"]["pow"]["seed"].is_string()
            && issued["challenge"]["rows"].is_array(),
        json!({"status":reply.status,"pow_difficulty":issued["preflight"]["pow"]["difficulty"]}),
    ));

    // The solve without its cryptographic preflight is refused with no receipt.
    let body = solve_body(&issued)?;
    let refused = pilot.post("/api/pass/solve", &cookie, &[], &body);
    rows.push(row(
        "missing-pow-refused",
        refused.status == Some(200)
            && refused.body["ok"] == false
            && refused.body["reason"] == "no cryptographic preflight"
            && refused.body.get("stepUpReceipt").is_none(),
        json!({"ok":refused.body["ok"],"reason":refused.body["reason"]}),
    ));

    let pow = pilot.pow(&cookie, &issued)?;
    rows.push(row(
        "proof-of-work-accepted",
        pow.status == Some(200) && pow.body["ok"] == true,
        json!({"status":pow.status,"ok":pow.body["ok"]}),
    ));

    let solved = pilot.post("/api/pass/solve", &cookie, &[], &body);
    let receipt = solved.body["stepUpReceipt"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    rows.push(row(
        "solve-accepted-with-receipt",
        solved.status == Some(200) && solved.body["ok"] == true && !receipt.is_empty(),
        json!({"ok":solved.body["ok"],"receipt_issued":!receipt.is_empty()}),
    ));

    let missing = pilot.redeem(&cookie, None);
    rows.push(row(
        "missing-receipt-refused",
        missing.status == Some(403) && missing.cookies.is_empty(),
        json!({"status":missing.status}),
    ));
    let mut forged = receipt.clone();
    let last = forged.pop().unwrap_or('A');
    forged.push(if last == 'A' { 'B' } else { 'A' });
    let forged = pilot.redeem(&cookie, Some(&forged));
    rows.push(row(
        "forged-receipt-refused",
        forged.status == Some(403) && forged.cookies.is_empty(),
        json!({"status":forged.status}),
    ));
    let (other, _) = pilot.pass_new(None);
    let other = other.ok_or("second Pass session cookie absent")?;
    let foreign = pilot.redeem(&other, Some(&receipt));
    rows.push(row(
        "wrong-session-receipt-refused",
        other != cookie && foreign.status == Some(403) && foreign.cookies.is_empty(),
        json!({"status":foreign.status}),
    ));

    let redeemed = pilot.redeem(&cookie, Some(&receipt));
    let proof = redeemed
        .cookies
        .iter()
        .find(|c| c.starts_with(&format!("{STEP_UP_COOKIE}=")))
        .cloned()
        .unwrap_or_default();
    rows.push(row(
        "receipt-redeemed",
        redeemed.status == Some(200) && !proof.is_empty(),
        json!({"status":redeemed.status,"proof_cookie_set":!proof.is_empty()}),
    ));
    let both = format!("{cookie}; {proof}");
    let (reply, delta) = pilot.attested(Some(&both), USER_AGENT, true)?;
    rows.push(row(
        "recovered-session-admitted",
        admitted(&reply, delta),
        observed(&reply, delta),
    ));

    // The proof is bound to the redeeming socket's header fingerprint.
    let (reply, delta) = pilot.attested(Some(&both), "Other/1.0", true)?;
    rows.push(row(
        "lifted-proof-refused",
        challenged(&reply, delta),
        observed(&reply, delta),
    ));
    let wrong = format!("{other}; {proof}");
    let (reply, delta) = pilot.attested(Some(&wrong), USER_AGENT, true)?;
    rows.push(row(
        "wrong-session-proof-refused",
        challenged(&reply, delta),
        observed(&reply, delta),
    ));
    // A proof never upgrades a scoring CHALLENGE (uncollected session).
    let (reply, delta) = pilot.attested(Some(&both), USER_AGENT, false)?;
    rows.push(row(
        "scoring-challenge-not-overridden",
        challenged(&reply, delta),
        observed(&reply, delta),
    ));
    // The solved body replayed against a fresh instance is stale.
    let (_, fresh) = pilot.pass_new(Some(&cookie));
    let replayed = pilot.post("/api/pass/solve", &cookie, &[], &body);
    rows.push(row(
        "replayed-solve-refused",
        fresh.status == Some(200)
            && replayed.body["ok"] == false
            && replayed.body["reason"] == "stale or missing challenge"
            && replayed.body.get("stepUpReceipt").is_none(),
        json!({"ok":replayed.body["ok"],"reason":replayed.body["reason"]}),
    ));

    // The genuine receipt, once past its expiry and the skew leeway.
    let expiry = receipt_expiry(&receipt);
    let started = Instant::now();
    let ready = expiry.is_some_and(|expiry| {
        loop {
            if unix_now() > expiry + RECEIPT_SKEW {
                break true;
            }
            if started.elapsed() > EXPIRY_BUDGET {
                break false;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
    let expired = pilot.redeem(&cookie, Some(&receipt));
    rows.push(row(
        "expired-receipt-refused",
        ready && expired.status == Some(403) && expired.cookies.is_empty(),
        json!({"status":expired.status,"waited_s":started.elapsed().as_secs(),"expiry_reached":ready}),
    ));

    // Positive control after every negative: the proof still admits.
    let (reply, delta) = pilot.attested(Some(&both), USER_AGENT, true)?;
    rows.push(row(
        "positive-control-after-negatives",
        admitted(&reply, delta),
        observed(&reply, delta),
    ));
    Ok(rows)
}

fn report() -> Result<Value> {
    let installation = Installation::new(false)?;
    let document = configuration(installation.config.clone(), WEB_DIR);
    let config: Configuration = serde_json::from_value(document.clone())?;
    let (runtime, address) = process::Runtime::start(&document);
    let pilot = Pilot {
        address,
        config,
        installation,
    };
    let rows = execute(&pilot);
    let effects = pilot.effects();
    drop(runtime);
    let rows = rows?;
    // The produced rows must be exactly the declared ledger.
    let exact = rows.len() == RECOVERY.len() + REFUSED.len()
        && RECOVERY
            .iter()
            .chain(REFUSED)
            .all(|id| rows.iter().filter(|row| row["id"] == *id).count() == 1);
    let passed = exact && rows.iter().all(|row| row["status"] == "passed");
    Ok(
        json!({"schema":SCHEMA,"status":if passed{"passed"}else{"failed"},"cases":rows,
        "origin_effects":effects,
        "client":"Rust HTTP/1.1 client inside this network-none container",
        "gate":"shipped humanymous-runtime binary, legacy Pass surface plus one attest_floor route",
        "accessibility":"keyboard-lane solve with no motor trace; no timing or motor gate exists",
        "not_qualified":["keyboard or assistive-technology human Pass run (human-attested)",
            "browser rendering of pass.html","scoring Pass-upgrade of a risk-band CHALLENGE on RIT routes",
            "separate Core and Gate hosts"],
        "full_product_qualified":false}),
    )
}

pub fn run() {
    let report = std::panic::catch_unwind(report)
        .map_err(|_| "pilot panicked".to_owned())
        .and_then(|result| result.map_err(|error| error.to_string()))
        .unwrap_or_else(|error| {
            json!({"schema":SCHEMA,"status":"failed","cases":[],"error":error,
                "full_product_qualified":false})
        });
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report).expect("pass report output");
    stdout.write_all(b"\n").expect("pass report output");
    if report["status"] != "passed" {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_core::scoring::engine::Engine;
    use humanymous_runtime::gateway::legacy::Surfaces;
    use std::path::Path;

    fn base() -> Value {
        crate::product_e2e::configuration("/audit", "/keys", "passphrase")
    }

    #[test]
    fn pilot_document_adds_only_the_attested_route_and_the_pass_surface() {
        let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web");
        let web = web.canonicalize().unwrap();
        let document = configuration(base(), web.to_str().unwrap());
        assert_eq!(document["legacy"]["pass"], true);
        assert!(document["legacy"].get("demo").is_none());
        let parsed: Configuration = serde_json::from_value(document).unwrap();
        assert!(parsed.validate().is_ok());
        let attested: Vec<_> = parsed.routes.iter().filter(|r| r.attest_floor).collect();
        assert_eq!(attested.len(), 1);
        assert_eq!(attested[0].path, ATTESTED);
        let legacy = Surfaces::new(parsed.legacy.as_ref().unwrap(), &Engine::default(), 4)
            .unwrap_or_else(|_| panic!("legacy surfaces refused"));
        for path in ["/pass", "/api/pass/new", "/api/pass/solve", STEP_UP_PATH] {
            assert!(legacy.mounted(path), "{path}");
        }
        assert!(!legacy.mounted("/demo"));
        assert!(Path::new(WEB_DIR).is_absolute());
    }

    #[test]
    fn ledger_ids_are_unique_and_disjoint() {
        let mut ids: Vec<_> = RECOVERY.iter().chain(REFUSED).collect();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total);
        assert_eq!(RECOVERY.last(), Some(&"positive-control-after-negatives"));
    }

    #[test]
    fn receipt_expiry_reads_only_the_public_payload() {
        let payload = URL_SAFE_NO_PAD.encode(b"sur|session|1700000000");
        assert_eq!(
            receipt_expiry(&format!("{payload}.mac")),
            Some(1_700_000_000)
        );
        let other = URL_SAFE_NO_PAD.encode(b"su|session|bind|1|e1");
        assert_eq!(receipt_expiry(&format!("{other}.mac")), None);
        assert_eq!(receipt_expiry("not base64!.x"), None);
    }

    #[test]
    fn solve_body_aligns_every_public_row() {
        let issued = json!({"bucket":7,"challengeNonce":"ab",
            "challenge":{"n":9,"center":4,"rows":[{"chars":["a"],"keyIndex":1},{"chars":["b"],"keyIndex":4},{"chars":["c"],"keyIndex":8}]}});
        let body: Value = serde_json::from_slice(&solve_body(&issued).unwrap()).unwrap();
        assert_eq!(body["offsets"], json!([3, 0, 5]));
        assert_eq!(body["challengeNonce"], "ab");
        assert!(body.get("keyDurs").is_none() && body.get("durations").is_none());
    }
}
