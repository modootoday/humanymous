//! Attacker/driver: real HTTP/1.1 bytes through the Gate, judged only by the
//! observed Gate status and the origin's own records. Session identifiers,
//! keys and tokens are never written to the receipt.
use crate::client::{report, request, sign, wire};
use humanymous_runtime::gateway::{configuration::Configuration, guard, http::Request};
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

fn status(response: &[u8]) -> Option<u16> {
    std::str::from_utf8(response.get(9..12)?).ok()?.parse().ok()
}

fn body(response: &[u8]) -> Option<Value> {
    let end = response.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    serde_json::from_slice(&response[end..]).ok()
}

fn records() -> Result<Vec<Value>> {
    let response = exchange(
        super::OBSERVE,
        b"GET /records HTTP/1.1\r\nHost: observe\r\n\r\n",
    )?;
    Ok(body(&response)
        .and_then(|v| v["records"].as_array().cloned())
        .ok_or("origin observation unavailable")?)
}

fn wait_for(address: &str) -> Result<()> {
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
fn forbidden(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "authorization"
        || lower == "proxy-authorization"
        || lower.starts_with("x-humanymous-")
        || guard::is_internal(name)
}

fn header_names(record: &Value) -> Vec<String> {
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
}

impl Driver {
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
        let mut collect = request("/_humanymous/collect", report(&issued));
        sign(&self.config, &mut collect, &issued, 1);
        let response = exchange(super::GATE, &wire(&collect))?;
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
        self.case(
            "forged_trust_headers_without_session",
            401,
            &wire(&forged),
            0,
        )?;
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
    let config: Configuration =
        serde_json::from_value(super::configuration("/unused", "/unused", "unused"))?;
    let mut driver = Driver {
        config,
        cases: Vec::new(),
    };
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
        "client_report":"client-asserted fixture report; no browser or collector executed"});
    println!("{receipt}");
    if failed > 0 || outcome.is_err() {
        return Err("product e2e assertions failed".into());
    }
    Ok(())
}
