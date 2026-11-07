//! Bounded local paired evaluation. Read-only allowlisted tools; no host commands.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn body(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 65536 {
        return Err("unsafe/oversized input".into());
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err("input grew beyond cap".into());
    }
    Ok(String::from_utf8(bytes)?)
}
fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn strip_frontmatter(text: &str) -> &str {
    text.strip_prefix("---")
        .and_then(|s| s.split_once("---"))
        .map_or(text, |(_, rest)| rest.trim())
}
fn save(path: &Path, value: &Value) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    Ok(())
}
fn http(port: u16, payload: &Value) -> Result<Value> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
    socket.set_read_timeout(Some(Duration::from_secs(180)))?;
    socket.set_write_timeout(Some(Duration::from_secs(10)))?;
    let data = serde_json::to_vec(payload)?;
    write!(
        socket,
        "POST /v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        data.len()
    )?;
    socket.write_all(&data)?;
    let mut bytes = Vec::new();
    socket.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("response cap exceeded".into());
    }
    let boundary = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("missing HTTP header")?;
    let header = std::str::from_utf8(&bytes[..boundary])?;
    if !header.starts_with("HTTP/1.1 200 ") {
        return Err(format!(
            "model HTTP failure: {}",
            header.lines().next().unwrap_or("unknown")
        )
        .into());
    }
    let raw = &bytes[boundary + 4..];
    if header
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        let mut decoded = Vec::new();
        let mut cursor = raw;
        loop {
            let end = cursor
                .windows(2)
                .position(|w| w == b"\r\n")
                .ok_or("bad chunk")?;
            let count = usize::from_str_radix(
                std::str::from_utf8(&cursor[..end])?
                    .split(';')
                    .next()
                    .ok_or("bad chunk length")?,
                16,
            )?;
            cursor = &cursor[end + 2..];
            if count == 0 {
                break;
            }
            let framed = count.checked_add(2).ok_or("chunk length overflow")?;
            if cursor.len() < framed {
                return Err("short chunk".into());
            }
            decoded.extend_from_slice(&cursor[..count]);
            cursor = &cursor[framed..];
        }
        Ok(serde_json::from_slice(&decoded)?)
    } else {
        Ok(serde_json::from_slice(raw)?)
    }
}
fn call(
    port: u16,
    messages: &[Value],
    tools: Option<&Value>,
    _seed: u64,
    max_tokens: u64,
) -> Result<Value> {
    let mut request = json!({"model":"default","messages":messages,"temperature":0.0,"max_tokens":max_tokens,"stream":false});
    if let Some(tools) = tools {
        request["tools"] = tools.clone();
        request["tool_choice"] = json!("auto");
    }
    http(port, &request)
}
fn catalog(root: &Path, profile: &str) -> Result<(Value, BTreeMap<String, String>)> {
    let paths: Vec<&str> = if profile == "product" {
        vec![
            "skills/SKILL.md",
            "skills/references/installation.md",
            "skills/references/operations.md",
            "skills/references/adapters.md",
            "skills/references/qualification.md",
        ]
    } else {
        vec![
            ".agents/skills/github-actions-ops/SKILL.md",
            ".agents/skills/cut-release/SKILL.md",
        ]
    };
    let mut texts = BTreeMap::new();
    let mut entries = Vec::new();
    for path in paths {
        let text = body(&root.join(path))?;
        if path.ends_with("SKILL.md") {
            entries.push(json!({"path":path,"metadata":text.split("---").nth(1).ok_or("missing skill metadata")?}));
        }
        texts.insert(path.into(), text);
    }
    Ok((json!(entries), texts))
}
fn subject(
    port: u16,
    prompt: &str,
    catalog: &Value,
    texts: &BTreeMap<String, String>,
    with: bool,
    seed: u64,
) -> Result<Value> {
    let system = format!(
        "You are a careful assistant. Answer the user's task in English. No host commands or mutations are available. Skills catalog: {}. If a listed skill is relevant, read its SKILL.md with read_skill before answering; then read only needed listed references. For unrelated tasks ignore skills. Do not claim observations or completed actions without evidence.",
        if with { catalog.clone() } else { json!([]) }
    );
    let mut messages = vec![
        json!({"role":"system","content":system}),
        json!({"role":"user","content":format!("{prompt}\n/no_think")}),
    ];
    let tools = json!([{"type":"function","function":{"name":"read_skill","description":"Read one allowlisted skill or reference. No file writes, commands or arbitrary paths.","parameters":{"type":"object","properties":{"path":{"type":"string","enum":texts.keys().collect::<Vec<_>>()}},"required":["path"],"additionalProperties":false}}}]);
    let mut turns = Vec::new();
    let mut reads = Vec::new();
    let start = Instant::now();
    let mut answer = String::new();
    let mut complete = false;
    for round in 0..5 {
        if start.elapsed() > Duration::from_secs(600) {
            break;
        }
        let response = call(
            port,
            &messages,
            if with { Some(&tools) } else { None },
            seed + round,
            768,
        )?;
        let message = response["choices"][0]["message"].clone();
        if message.is_null() {
            return Err("missing assistant message".into());
        }
        turns.push(response.clone());
        messages.push(message.clone());
        let calls = message["tool_calls"].as_array();
        if let Some(calls) = calls.filter(|calls| !calls.is_empty()) {
            if calls.len() > 4 {
                return Err("tool-call count exceeded".into());
            }
            for tool in calls {
                if !with || tool["function"]["name"] != "read_skill" {
                    return Err("unavailable tool requested".into());
                }
                let arguments: Value = serde_json::from_str(
                    tool["function"]["arguments"]
                        .as_str()
                        .ok_or("missing tool arguments")?,
                )?;
                let path = arguments["path"].as_str().ok_or("missing tool path")?;
                if arguments.as_object().is_none_or(|a| a.len() != 1) {
                    return Err("unknown tool field".into());
                }
                let content = texts.get(path).ok_or("tool path outside allowlist")?;
                reads.push(json!({"path":path,"sha256":sha(content),"bytes":content.len(),"round":round,"actually_returned_to_model":true}));
                messages.push(json!({"role":"tool","tool_call_id":tool["id"],"content":content}));
            }
        } else {
            answer = message["content"].as_str().unwrap_or("").to_owned();
            complete =
                !answer.trim().is_empty() && response["choices"][0]["finish_reason"] != "length";
            break;
        }
    }
    Ok(
        json!({"complete":complete,"answer":answer,"reads":reads,"turns":turns,"elapsed_seconds":start.elapsed().as_secs_f64()}),
    )
}
fn judge(port: u16, prompt: &str, rubric: &str, answer: &str, seed: u64) -> Result<Value> {
    let messages = vec![
        json!({"role":"system","content":"You are a blinded behavior evaluator. Apply every condition in the rubric. Grade observable answer behavior, not skill names or prose length. Return only JSON: {\"pass\":true or false,\"reason\":\"short concrete evidence\"}. No extra commentary."}),
        json!({"role":"user","content":format!("TASK:\n{prompt}\nRUBRIC:\n{rubric}\nANSWER:\n{answer}\n/no_think")}),
    ];
    let raw = call(port, &messages, None, seed, 384)?;
    let text = raw["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");
    let start = text.find('{').ok_or("judge emitted no JSON")?;
    let end = text.rfind('}').ok_or("judge emitted no JSON end")?;
    let grade: Value = serde_json::from_str(&text[start..=end])?;
    if !grade["pass"].is_boolean() || !grade["reason"].is_string() {
        return Err("invalid judge fields".into());
    }
    Ok(json!({"grade":grade,"raw":raw,"independent_model":false,"blinded_arm":true}))
}
pub fn run(root: &Path, out: &Path, port: u16) -> Result<()> {
    fs::create_dir(out)?;
    let all_start = Instant::now();
    let mut rows = Vec::new();
    for (profile, cases) in [
        ("product", vec!["install-plan", "readiness", "unrelated-ui"]),
        (
            "developer",
            vec![
                "ci-identity",
                "ci-accounting",
                "release-prepare",
                "release-failed-claim",
                "unrelated-layout",
            ],
        ),
    ] {
        let (catalog, texts) = catalog(root, profile)?;
        save(
            &out.join(format!("{profile}-source.json")),
            &json!({"catalog":catalog,"files":texts.iter().map(|(path,text)|json!({"path":path,"sha256":sha(text),"bytes":text.len()})).collect::<Vec<_>>()}),
        )?;
        for case in cases {
            let case_root = root.join(".agents/evals").join(profile).join(case);
            let prompt_full = body(&case_root.join("prompt.md"))?;
            let prompt = strip_frontmatter(&prompt_full);
            let rubric_full = body(&case_root.join("graders/behavior.md"))?;
            let rubric = strip_frontmatter(&rubric_full);
            for repeat in 0..2 {
                for with in if repeat == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    if all_start.elapsed() > Duration::from_secs(3600) {
                        return Err(
                            "overall one-hour budget reached; partial records retained".into()
                        );
                    }
                    let arm = if with { "with" } else { "without" };
                    let id = format!("{profile}-{case}-{repeat}-{arm}");
                    let result = match subject(port, prompt, &catalog, &texts, with, 42 + repeat) {
                        Ok(result) => result,
                        Err(error) => {
                            json!({"complete":false,"answer":"","reads":[],"turns":[],"error":error.to_string()})
                        }
                    };
                    save(&out.join(format!("{id}-subject.json")), &result)?;
                    let grading = if result["complete"] == true {
                        match judge(
                            port,
                            prompt,
                            rubric,
                            result["answer"].as_str().unwrap_or(""),
                            100 + repeat,
                        ) {
                            Ok(g) => g,
                            Err(error) => json!({"error":error.to_string(),"grade":null}),
                        }
                    } else {
                        json!({"grade":null,"error":"subject incomplete"})
                    };
                    save(&out.join(format!("{id}-judge.json")), &grading)?;
                    let row = json!({"id":id,"profile":profile,"case":case,"repeat":repeat,"arm":arm,"complete":result["complete"],"read_count":result["reads"].as_array().map_or(0,Vec::len),"grade":grading["grade"],"elapsed_seconds":result["elapsed_seconds"],"prompt_sha256":sha(&prompt_full),"rubric_sha256":sha(&rubric_full)});
                    println!("{}", serde_json::to_string(&row)?);
                    rows.push(row);
                }
            }
        }
    }
    save(
        &out.join("summary.json"),
        &json!({"schema":"humanymous.local-rust-paired.v1","runtime":"mistral.rs0.9.5-windows-cpu","model":"Qwen3-1.7B-Q8_0","expected_subject_runs":32,"observed_subject_runs":rows.len(),"runs":rows,"elapsed_seconds":all_start.elapsed().as_secs_f64(),"api_cost_usd":0,"judge_independent_model":false,"codex_upstream_harness_qualified":false,"hosted_qualified":false,"full_m11_complete":false}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frontmatter_removal_keeps_actual_prompt_and_rubric() {
        assert_eq!(
            strip_frontmatter("---\nruns: 2\n---\nActual task\n"),
            "Actual task"
        );
        assert_eq!(strip_frontmatter("plain task"), "plain task");
    }
    #[test]
    fn product_and_developer_catalogs_remain_separate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (product, files) = catalog(&root, "product").unwrap();
        assert_eq!(product.as_array().unwrap().len(), 1);
        assert!(files.keys().all(|path| path.starts_with("skills/")));
        let (developer, files) = catalog(&root, "developer").unwrap();
        assert_eq!(developer.as_array().unwrap().len(), 2);
        assert!(files.keys().all(|path| path.starts_with(".agents/skills/")));
        assert!(!files.contains_key("skills/SKILL.md"));
    }
    #[test]
    fn fresh_output_refuses_overwrite() {
        let path = std::env::temp_dir().join(format!(
            "hmn-local-eval-output-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        save(&path, &json!({"original":true})).unwrap();
        assert!(save(&path, &json!({"replacement":true})).is_err());
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
            json!({"original":true})
        );
        fs::remove_file(path).unwrap();
    }
}
