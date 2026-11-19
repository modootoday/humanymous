//! Attacker/driver: real HTTP/1.1 bytes through the Gate, judged only by the
//! observed Gate status and the origin's own records. Session identifiers,
//! keys and tokens are never written to the receipt.
use super::report::{self, Captured};
use crate::client::{request, sign, wire};
use humanymous_runtime::gateway::{
    ban::gate::Mode, configuration::Configuration, guard, http::Request,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn exchange(address: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&super::address(address), Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(bytes)?;
    let mut result = Vec::new();
    stream.read_to_end(&mut result)?;
    Ok(result)
}

pub(super) fn status(response: &[u8]) -> Option<u16> {
    std::str::from_utf8(response.get(9..12)?).ok()?.parse().ok()
}

pub(super) fn body(response: &[u8]) -> Option<Value> {
    let end = response.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    serde_json::from_slice(&response[end..]).ok()
}

/// Exact body bytes after the response head, for non-JSON Gate bodies.
fn raw_body(response: &[u8]) -> Option<&[u8]> {
    let end = response.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    response.get(end..)
}

/// Status and exact Gate body bytes a sessionless forged-trust-header request
/// must get under the configured spoof-header mode (`application/edge.rs`).
/// The refusal is written in Go `http.Error` form: the static deny body plus
/// one newline (`http::text_frame`).
fn spoof_expectation(mode: Mode) -> (u16, Option<Vec<u8>>) {
    match mode {
        Mode::Enforce => (
            403,
            Some(format!("{}\n", guard::SPOOF_DENY_BODY).into_bytes()),
        ),
        Mode::Off | Mode::Monitor | Mode::Shadow => (401, None),
    }
}

pub(super) fn records() -> Result<Vec<Value>> {
    let response = exchange(
        super::OBSERVE,
        b"GET /records HTTP/1.1\r\nHost: observe\r\n\r\n",
    )?;
    Ok(body(&response)
        .and_then(|v| v["records"].as_array().cloned())
        .ok_or("origin observation unavailable")?)
}

pub(super) fn wait_for(address: &str) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(90);
    while TcpStream::connect_timeout(&super::address(address), Duration::from_secs(1)).is_err() {
        if Instant::now() > until {
            return Err(format!("{address} not reachable").into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}

/// Header names an origin must never receive from a client through the Gate.
pub(super) fn forbidden(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "authorization"
        || lower == "proxy-authorization"
        || lower.starts_with("x-humanymous-")
        || guard::is_internal(name)
}

pub(super) fn header_names(record: &Value) -> Vec<String> {
    record["headers"]
        .as_array()
        .map(|headers| {
            headers
                .iter()
                .filter_map(|pair| pair[0].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

struct Driver {
    config: Configuration,
    cases: Vec<Value>,
    /// The real collector report from the browser topology; never a fixture.
    report: std::result::Result<Captured, String>,
    /// Every collect exchange: the Gate's status and verdict, as observed.
    collects: Vec<Value>,
}

impl Driver {
    /// Read the captured collector report the runner supplies on stdin.
    fn new() -> Result<Self> {
        Ok(Self {
            config: driver_configuration()?,
            cases: Vec::new(),
            report: report::from_stdin(),
            collects: Vec::new(),
        })
    }

    /// Receipt description of the report every collected session submitted.
    fn client_report(&self) -> Value {
        match &self.report {
            Ok(captured) => json!({"kind":"real Rust WASM collector report replayed",
                "provenance":captured.provenance,"collect_observations":self.collects}),
            Err(error) => json!({"kind":"absent","error":error,"fixture_fallback":false}),
        }
    }

    /// Send bytes; record expected/observed status and the origin delta.
    fn case(
        &mut self,
        id: &str,
        expected: u16,
        bytes: &[u8],
        forwarded: usize,
    ) -> Result<(Option<u16>, Vec<Value>)> {
        let before = records()?.len();
        let response = exchange(super::GATE, bytes);
        let after = records()?;
        let new: Vec<Value> = after[before..].to_vec();
        let observed = response.as_deref().ok().and_then(status);
        let local = response.as_deref().ok().and_then(body);
        let pass = observed == Some(expected) && new.len() == forwarded;
        self.cases.push(json!({"id":id,"expected_status":expected,"observed_status":observed,
            "expected_origin_delta":forwarded,"origin_delta":new.len(),
            "gate_local_body":if forwarded == 0 { local.unwrap_or(Value::Null) } else { Value::Null },
            "transport_error":response.as_ref().err().map(|e|e.to_string()),"pass":pass}));
        Ok((observed, new))
    }

    /// Forged trust headers on a sessionless request. The expectation follows
    /// the `edge.spoof_header` mode of the one configuration document the Gate
    /// was started with: an enforcing spoof gate answers 403 with its static
    /// deny body before the session check; any other mode strips the claims
    /// and the session check answers 401. Either way the origin sees nothing.
    /// The runner cross-checks the recorded mode against the Gate's readiness.
    fn forged_without_session(&mut self, bytes: &[u8]) -> Result<()> {
        let mode = self.config.edge.spoof_header;
        let (expected, deny_body) = spoof_expectation(mode);
        let before = records()?.len();
        let response = exchange(super::GATE, bytes);
        let origin_delta = records()?.len() - before;
        let observed = response.as_deref().ok().and_then(status);
        let local = response.as_deref().ok().and_then(raw_body);
        let body_exact = deny_body.as_deref().is_none_or(|want| local == Some(want));
        let pass = observed == Some(expected) && body_exact && origin_delta == 0;
        // The deny body is a fixed public string, so the observed bytes are
        // recorded as text; a JSON Gate body is recorded as JSON.
        let observed_body = match deny_body {
            Some(_) => local.map_or(Value::Null, |b| json!(String::from_utf8_lossy(b))),
            None => response
                .as_deref()
                .ok()
                .and_then(body)
                .unwrap_or(Value::Null),
        };
        self.cases
            .push(json!({"id":"forged_trust_headers_without_session",
            "spoof_header_mode":self.config.edge.readiness()["spoof_header"],
            "expected_status":expected,"observed_status":observed,
            "expected_gate_body":deny_body.map(|b| String::from_utf8_lossy(&b).into_owned()),
            "gate_body_exact":body_exact,"gate_local_body":observed_body,
            "expected_origin_delta":0,"origin_delta":origin_delta,
            "transport_error":response.as_ref().err().map(|e|e.to_string()),"pass":pass}));
        Ok(())
    }

    fn session(&mut self) -> Result<Value> {
        let response = exchange(super::GATE, &wire(&request("/_humanymous/session", vec![])))?;
        let issued = body(&response).ok_or("session body")?;
        if status(&response) != Some(200)
            || !issued["session"].is_string()
            || !issued["key"].is_string()
        {
            return Err("session issue refused".into());
        }
        Ok(issued)
    }

    fn collected(&mut self) -> Result<Value> {
        let issued = self.session()?;
        let entity = match &self.report {
            Ok(captured) => captured.bind(issued["session"].as_str().ok_or("session id")?),
            Err(error) => return Err(format!("no real collector report: {error}").into()),
        };
        let mut collect = request("/_humanymous/collect", entity);
        sign(&self.config, &mut collect, &issued, 1);
        let response = exchange(super::GATE, &wire(&collect))?;
        // Observation, not an expectation: what the Gate made of the report.
        self.collects
            .push(json!({"observed_status":status(&response),
            "gate_verdict":body(&response).map_or(Value::Null, |b| b["verdict"].clone())}));
        if status(&response) != Some(200) {
            return Err(format!("collect refused: {:?}", status(&response)).into());
        }
        Ok(issued)
    }

    /// A signed request whose verdict is the Gate's own decision. The headers
    /// assertion passes when no forbidden header reaches the origin, and the
    /// receipt states whether the strip path was actually exercised.
    fn signed_headers(&mut self, id: &str, extra: &[(&str, &str)]) -> Result<()> {
        let issued = self.collected()?;
        let mut req = request("/effect", b"effect".to_vec());
        for (name, value) in extra {
            req.headers.push(((*name).into(), (*value).into()));
        }
        sign(&self.config, &mut req, &issued, 2);
        let before = records()?.len();
        let response = exchange(super::GATE, &wire(&req));
        let new = records()?[before..].to_vec();
        let observed = response.as_deref().ok().and_then(status);
        let leaked: Vec<String> = new
            .iter()
            .flat_map(header_names)
            .filter(|name| forbidden(name))
            .collect();
        let pass = leaked.is_empty() && (observed == Some(200)) == (new.len() == 1);
        self.cases.push(json!({"id":id,"sent_headers":extra.iter().map(|(n,_)|*n).collect::<Vec<_>>(),
            "observed_status":observed,"origin_delta":new.len(),"strip_path_exercised":new.len()==1,
            "origin_header_names":new.first().map(header_names),"forbidden_headers_at_origin":leaked,
            "gate_local_body":if new.is_empty(){response.as_deref().ok().and_then(body)}else{None},"pass":pass}));
        Ok(())
    }

    fn signed(&mut self, path: &str, payload: &[u8], mutate: bool) -> Result<Request> {
        let issued = self.collected()?;
        let mut req = request(path, payload.to_vec());
        sign(&self.config, &mut req, &issued, 2);
        if mutate {
            req.body[0] = b'X';
        }
        Ok(req)
    }

    fn run(&mut self) -> Result<()> {
        let plain = wire(&request("/effect", b"effect".to_vec()));
        self.case("no_session_protected_route", 401, &plain, 0)?;
        let mut forged = request("/effect", b"effect".to_vec());
        for (name, value) in FORGED {
            forged.headers.push(((*name).into(), (*value).into()));
        }
        self.forged_without_session(&wire(&forged))?;
        let outside = wire(&request("/not-in-inventory", b"effect".to_vec()));
        self.case("route_outside_inventory", 409, &outside, 0)?;
        let admin = wire(&request("/_humanymous/admin", vec![]));
        self.case("reserved_admin_route", 403, &admin, 0)?;
        let mut foreign = request("/effect", b"effect".to_vec());
        foreign.headers[0].1 = "foreign.test".into();
        self.case("foreign_host_authority", 403, &wire(&foreign), 0)?;
        for (id, fields) in SMUGGLING {
            let frame = format!("POST /effect HTTP/1.1\r\nHost: owned.test\r\n{fields}\r\n\r\n");
            self.case(id, 400, frame.as_bytes(), 0)?;
        }
        self.case(
            "missing_raw_tls_capability",
            422,
            &wire(&request("/tls", vec![])),
            0,
        )?;
        let collector = wire(&request("/collector", vec![]));
        self.case("missing_collector_capability", 422, &collector, 0)?;

        let clean = self.signed("/effect", b"effect", false)?;
        let clean_bytes = wire(&clean);
        let (_, new) = self.case("signed_session_effect_forwarded", 200, &clean_bytes, 1)?;
        if new.is_empty() {
            // Recorded, never silently dropped: with no forwarded request the
            // origin exactness check was not exercised and cannot pass.
            self.cases.push(
                json!({"id":"origin_observed_exact_request","origin_records":0,
                "exercised":false,"pass":false}),
            );
        }
        if let Some(record) = new.first() {
            let names = header_names(record);
            let host = record["headers"]
                .as_array()
                .and_then(|h| h.iter().find(|p| p[0] == "Host"))
                .and_then(|p| p[1].as_str());
            let exact = record["method"] == "POST"
                && record["target"] == "/effect"
                && record["body_length"] == 6
                && host == Some("owned.test")
                && !names.iter().any(|n| forbidden(n));
            self.cases.push(json!({"id":"origin_observed_exact_request","origin_header_names":names,
                "origin_method":record["method"],"origin_target":record["target"],
                "origin_body_length":record["body_length"],"origin_body_sha256":record["body_sha256"],"pass":exact}));
        }
        self.case("replay_of_forwarded_request", 403, &clean_bytes, 0)?;
        let mutated = wire(&self.signed("/effect", b"effect", true)?);
        self.case("signed_body_mutation", 403, &mutated, 0)?;
        let issued = self.session()?;
        let mut uncollected = request("/effect", b"effect".to_vec());
        sign(&self.config, &mut uncollected, &issued, 1);
        self.case("uncollected_session_effect", 403, &wire(&uncollected), 0)?;
        self.signed_headers("signed_forged_trust_headers", FORGED)?;
        // One header per session: the combined set is a scoring input, so each
        // claim alone shows which ones reach the origin strip path at all.
        for pair in FORGED {
            let id = format!(
                "signed_forged_{}",
                pair.0.to_ascii_lowercase().replace('-', "_")
            );
            self.signed_headers(&id, std::slice::from_ref(pair))?;
        }
        self.signed_headers(
            "signed_client_x_humanymous_header",
            &[("X-Humanymous-Verdict", "ALLOW")],
        )?;
        self.signed_headers(
            "signed_benign_header_control",
            &[("X-Lab-Marker", "benign")],
        )?;
        Ok(())
    }
}

/// Forged trust claims a client must never be able to hand to the origin.
const FORGED: &[(&str, &str)] = &[
    ("X-Forwarded-For", "203.0.113.7"),
    ("Forwarded", "for=203.0.113.7"),
    ("X-Real-Ip", "203.0.113.7"),
    ("X-Hmny-Origin-Auth", "forged"),
    ("X-Hmn-Verdict", "ALLOW"),
    ("Cf-Connecting-Ip", "203.0.113.7"),
];

const SMUGGLING: &[(&str, &str)] = &[
    (
        "smuggling_cl_te",
        "Content-Length: 6\r\nTransfer-Encoding: chunked",
    ),
    (
        "smuggling_duplicate_cl",
        "Content-Length: 6\r\nContent-Length: 7",
    ),
    (
        "smuggling_identical_duplicate_cl",
        "Content-Length: 6\r\nContent-Length: 6",
    ),
    ("smuggling_te_chunked_alone", "Transfer-Encoding: chunked"),
    (
        "smuggling_obs_fold",
        "Content-Length: 0\r\nX-Custom: ok\r\n Injected: yes",
    ),
];

/// Drive every case, print one receipt line, and fail on any failed assertion.
pub fn run() -> Result<()> {
    wait_for(super::OBSERVE)?;
    wait_for(super::GATE)?;
    let mut driver = Driver::new()?;
    let outcome = driver.run();
    let all = records()?;
    let leaked: Vec<String> = all
        .iter()
        .flat_map(header_names)
        .filter(|name| forbidden(name))
        .collect();
    let malformed = all.iter().filter(|r| r["malformed"] == true).count();
    driver
        .cases
        .push(json!({"id":"global_no_forbidden_header_at_origin",
        "origin_records_total":all.len(),"malformed_origin_contacts":malformed,
        "forbidden_headers_at_origin":leaked,"pass":leaked.is_empty() && malformed == 0}));
    // Topology observation, not a Gate assertion: the shared namespace leaves
    // the origin socket reachable from the attacker container.
    let bypass =
        TcpStream::connect_timeout(&super::address(super::ORIGIN), Duration::from_secs(2)).is_ok();
    let failed = driver.cases.iter().filter(|c| c["pass"] != true).count();
    let receipt = json!({"schema":"humanymous.product-e2e-driver/1",
        "cases":driver.cases,"passed":driver.cases.len()-failed,"failed":failed,
        "driver_error":outcome.as_ref().err().map(|e|e.to_string()),
        "observations":{"origin_socket_reachable_from_attacker_namespace":bypass},
        "client_report":driver.client_report()});
    println!("{receipt}");
    if failed > 0 || outcome.is_err() {
        return Err("product e2e assertions failed".into());
    }
    Ok(())
}

fn driver_configuration() -> Result<Configuration> {
    Ok(serde_json::from_value(super::configuration(
        "/unused", "/unused", "unused",
    ))?)
}

/// Phase marker the runner waits for before it removes a dependency.
pub const AWAITING_LOSS: &str = "awaiting_dependency_loss";
/// Exact line the runner sends after it observed the dependency stopped.
pub const DEPENDENCY_LOST: &str = "dependency-lost";

/// Witness-required Gate: a witnessed effect, then (after the runner stops the
/// witness container) a pre-signed effect from an already collected session
/// must fail closed: no success status and no origin effect.
pub fn witnessed() -> Result<()> {
    wait_for(super::OBSERVE)?;
    wait_for(super::GATE)?;
    let mut driver = Driver::new()?;
    let outcome = witnessed_cases(&mut driver);
    finish(driver, outcome, "humanymous.product-e2e-witnessed-driver/1")
}

fn witnessed_cases(driver: &mut Driver) -> Result<()> {
    let plain = wire(&request("/effect", b"effect".to_vec()));
    driver.case("witnessed_no_session_protected_route", 401, &plain, 0)?;
    let clean = wire(&driver.signed("/effect", b"effect", false)?);
    driver.case("witnessed_signed_effect_forwarded", 200, &clean, 1)?;
    // Prepared before the loss: session issue and collect are already audited.
    let pending = wire(&driver.signed("/effect", b"effect", false)?);
    println!(
        "{}",
        json!({"schema":"humanymous.product-e2e-driver-phase/1","phase":AWAITING_LOSS})
    );
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    if line.trim_end() != DEPENDENCY_LOST {
        return Err("runner did not confirm dependency loss".into());
    }
    driver.fails_closed("witness_lost_presigned_effect_fails_closed", &pending)?;
    // Observation only: whether session issue still answers after the loss.
    let issued = exchange(super::GATE, &wire(&request("/_humanymous/session", vec![])));
    driver
        .cases
        .push(json!({"id":"witness_lost_session_issue_observed",
        "observed_status":issued.as_deref().ok().and_then(status),
        "transport_error":issued.as_ref().err().map(|e|e.to_string()),
        "observation_only":true,"pass":true}));
    // A failed durable append poisons the session epoch, so the status of an
    // unsigned request is recorded, not predicted; it must still fail closed.
    driver.fails_closed("witness_lost_unsigned_request_fails_closed", &plain)
}

impl Driver {
    /// Dependency-loss judgement: any non-2xx answer (or no answer) with no
    /// origin record. The observed status and Gate body are recorded as facts.
    fn fails_closed(&mut self, id: &str, bytes: &[u8]) -> Result<()> {
        let before = records()?.len();
        let response = exchange(super::GATE, bytes);
        let new = records()?.len() - before;
        let observed = response.as_deref().ok().and_then(status);
        let closed = !observed.is_some_and(|code| (200..300).contains(&code)) && new == 0;
        self.cases.push(
            json!({"id":id,"observed_status":observed,"origin_delta":new,
            "gate_local_body":response.as_deref().ok().and_then(body),
            "transport_error":response.as_ref().err().map(|e|e.to_string()),
            "expectation":"no 2xx status and no origin effect","pass":closed}),
        );
        Ok(())
    }
}

/// Gate after its origin container was stopped post-readiness: a signed effect
/// must not report success, and the origin must actually be unreachable.
pub fn origin_lost() -> Result<()> {
    wait_for(super::GATE)?;
    let mut driver = Driver::new()?;
    let unreachable = |address| {
        TcpStream::connect_timeout(&super::address(address), Duration::from_secs(2)).is_err()
    };
    let gone = unreachable(super::ORIGIN) && unreachable(super::OBSERVE);
    driver
        .cases
        .push(json!({"id":"origin_absent_after_readiness","origin_unreachable":gone,"pass":gone}));
    let outcome = (|| -> Result<()> {
        let effect = wire(&driver.signed("/effect", b"effect", false)?);
        let response = exchange(super::GATE, &effect);
        let observed = response.as_deref().ok().and_then(status);
        let closed = observed.is_some_and(|code| code >= 500);
        driver
            .cases
            .push(json!({"id":"origin_lost_signed_effect_fails_closed",
            "observed_status":observed,"gate_local_body":response.as_deref().ok().and_then(body),
            "transport_error":response.as_ref().err().map(|e|e.to_string()),
            "expectation":"Gate answers with a 5xx; no success is reported","pass":closed}));
        Ok(())
    })();
    finish(
        driver,
        outcome,
        "humanymous.product-e2e-origin-lost-driver/1",
    )
}

fn finish(driver: Driver, outcome: Result<()>, schema: &str) -> Result<()> {
    let failed = driver.cases.iter().filter(|c| c["pass"] != true).count();
    let receipt = json!({"schema":schema,"cases":driver.cases,
        "passed":driver.cases.len()-failed,"failed":failed,
        "driver_error":outcome.as_ref().err().map(|e|e.to_string()),
        "client_report":driver.client_report()});
    println!("{receipt}");
    if failed > 0 || outcome.is_err() {
        return Err("product e2e assertions failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forged_header_expectation_follows_the_configured_spoof_mode() {
        assert_eq!(
            spoof_expectation(Mode::Enforce),
            (
                403,
                Some(b"Request blocked by humanymous (automation detected).\n".to_vec())
            )
        );
        for mode in [Mode::Off, Mode::Monitor, Mode::Shadow] {
            assert_eq!(spoof_expectation(mode), (401, None));
        }
        // The lab configuration document sets no edge section, so the Gate
        // and the driver both resolve the runtime's enforcing default.
        let config = driver_configuration().unwrap();
        assert_eq!(config.edge.spoof_header, Mode::Enforce);
        assert_eq!(config.edge.readiness()["spoof_header"], "enforce");
    }

    #[test]
    fn raw_body_is_the_exact_bytes_after_the_head() {
        let response = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 3\r\n\r\nabc";
        assert_eq!(raw_body(response), Some(&b"abc"[..]));
        assert_eq!(raw_body(b"HTTP/1.1 403 Forbidden\r\n"), None);
        assert_eq!(status(response), Some(403));
    }
}
