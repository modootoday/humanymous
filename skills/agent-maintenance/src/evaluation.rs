//! Small real CLI pilot, separate from the upstream Linux/bwrap plugin harness.
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const CASES: [(&str, &str, bool); 3] = [
    (
        "rust-slice",
        "Implement the accepted SoT slice for a Rust authority boundary. A mock succeeds but the real peer pilot is missing. Give the next implementation and verification plan.",
        true,
    ),
    (
        "wasm-slice",
        "Implement the accepted SoT slice for the browser collector. Native units pass, but we have not run the actual browser. State the authored language and necessary verification.",
        true,
    ),
    (
        "explain-only",
        "Explain conceptually what a TLS fingerprint is. Do not modify code, implement a feature, run tests, or write a design specification.",
        false,
    ),
];

#[derive(Serialize)]
struct Row {
    case: String,
    arm: String,
    run: usize,
    process_ok: bool,
    timed_out: bool,
    structured_answer: bool,
    quality: Option<bool>,
    claimed_skill: Option<String>,
    observed_skill_read: bool,
    skill_read_blocked: bool,
    tools_observed: usize,
    elapsed_millis: u128,
    tokens: u64,
    trace_sha256: String,
}
#[derive(Serialize)]
struct Report {
    schema_version: u8,
    harness: &'static str,
    suite: String,
    runtime_version: String,
    sandbox: &'static str,
    default_model: bool,
    runs_per_arm: usize,
    max_calls: usize,
    max_observed_tokens: u64,
    partial: bool,
    partial_reason: Option<String>,
    upstream_harness_status: &'static str,
    rows: Vec<Row>,
}

fn digest(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!("{:x}", sha2::Sha256::digest(bytes))
}

fn grade(value: &Value, positive: bool, wasm: bool) -> bool {
    let plan = value
        .get("plan")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    if !positive {
        return !plan.is_empty() && value.get("skill").and_then(Value::as_str) == Some("none");
    }
    let rust = plan.contains("rust") && plan.contains("1.99.1") && plan.contains("2024");
    let tests = plan.contains("cargo")
        && plan.contains("locked")
        && (plan.contains("clippy") || plan.contains("lint"));
    let actual = if wasm {
        plan.contains("browser") && (plan.contains("isolat") || plan.contains("docker"))
    } else {
        plan.contains("peer") && (plan.contains("actual") || plan.contains("real"))
    };
    rust && tests && actual
}

fn observed(trace: &str) -> (bool, usize, u64) {
    let mut read = false;
    let mut tools = 0;
    let mut tokens = 0;
    for line in trace.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(item) = v.get("item") {
            let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
            if matches!(kind, "command_execution" | "mcp_tool_call") {
                if v.get("type").and_then(Value::as_str) == Some("item.completed") {
                    tools += 1;
                }
                let command = item.get("command").and_then(Value::as_str).unwrap_or("");
                read |= command.contains("implement-sot-slice") && command.contains("SKILL.md");
            }
        }
        if let Some(usage) = v.get("usage") {
            tokens += usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + usage
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
        }
    }
    (read, tools, tokens)
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn run(
    root: &Path,
    cli: &Path,
    output: &Path,
    suite: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // No overwrite or shell interpolation. Fresh fixture root is outside the repo,
    // so its baseline cannot inherit this repository's AGENTS or skills.
    fs::create_dir(output)?;
    let output = output.canonicalize()?;
    let version = Command::new(cli).arg("--version").output()?;
    if !version.status.success() {
        return Err("CLI version query failed".into());
    }
    let mut report = Report {
        schema_version: 1,
        harness: "rust-cli-skill-pilot-v1",
        suite: suite.into(),
        runtime_version: String::from_utf8_lossy(&version.stdout).trim().into(),
        sandbox: "read-only",
        default_model: true,
        runs_per_arm: 2,
        max_calls: 12,
        max_observed_tokens: 100_000,
        partial: false,
        partial_reason: None,
        upstream_harness_status: "separate-pilot: complete upstream plugin/shared package required; no upstream qualification",
        rows: Vec::new(),
    };
    let scratch = Scratch(std::env::temp_dir().join(format!(
            "humanymous-agent-eval-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        )));
    fs::create_dir(&scratch.0)?;
    let schema = output.join("answer.schema.json");
    fs::write(
        &schema,
        serde_json::to_vec(
            &serde_json::json!({"type":"object","properties":{"skill":{"type":"string"},"plan":{"type":"string"}},"required":["skill","plan"],"additionalProperties":false}),
        )?,
    )?;
    let mut tokens = 0;
    'cases: for (case, task, positive) in CASES {
        for arm in ["without", "with"] {
            for run in 1..=2 {
                if tokens >= report.max_observed_tokens {
                    report.partial = true;
                    report.partial_reason = Some("max-observed-tokens".into());
                    break 'cases;
                }
                let id = format!("{case}-{arm}-{run}");
                let work = scratch.0.join(&id);
                fs::create_dir(&work)?;
                if arm == "with" {
                    let skill = work.join(".agents/skills/implement-sot-slice");
                    fs::create_dir_all(&skill)?;
                    fs::copy(
                        root.join(".agents/skills/implement-sot-slice/SKILL.md"),
                        skill.join("SKILL.md"),
                    )?;
                }
                let trace = output.join(format!("{id}.jsonl"));
                let stderr = output.join(format!("{id}.stderr.txt"));
                let answer = output.join(format!("{id}.answer.json"));
                let prompt = format!(
                    "This is a bounded planning evaluation. Do not implement, modify files, browse, call external services or run tests. You may read a relevant local SKILL.md if discovered. Return JSON with skill (the relevant local skill name, or none) and plan (up to 160 words). Use the local skill if relevant; do not invent one if absent. Task: {task}"
                );
                let start = Instant::now();
                let mut child = Command::new(cli)
                    .args([
                        "exec",
                        "--ignore-user-config",
                        "--ephemeral",
                        "--sandbox",
                        "read-only",
                        "--skip-git-repo-check",
                        "--color",
                        "never",
                        "--json",
                        "--output-schema",
                    ])
                    .arg(&schema)
                    .arg("--output-last-message")
                    .arg(&answer)
                    .arg("-C")
                    .arg(&work)
                    .arg(&prompt)
                    .stdin(Stdio::null())
                    .stdout(fs::File::create(&trace)?)
                    .stderr(fs::File::create(&stderr)?)
                    .spawn()?;
                let mut timed_out = false;
                let status = loop {
                    if let Some(status) = child.try_wait()? {
                        break status;
                    }
                    if start.elapsed() > Duration::from_secs(90) {
                        timed_out = true;
                        child.kill()?;
                        break child.wait()?;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                };
                let raw = fs::read(&trace)?;
                let text = String::from_utf8_lossy(&raw);
                let (read, tools, used) = observed(&text);
                tokens += used;
                let value = fs::read(&answer)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
                let blocked = fs::read_to_string(stderr)?.contains("blocked by policy");
                report.rows.push(Row {
                    case: case.into(),
                    arm: arm.into(),
                    run,
                    process_ok: status.success(),
                    timed_out,
                    structured_answer: value.is_some(),
                    quality: value
                        .as_ref()
                        .filter(|_| !blocked)
                        .map(|v| grade(v, positive, case == "wasm-slice")),
                    claimed_skill: value
                        .as_ref()
                        .and_then(|v| v.get("skill"))
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    observed_skill_read: read,
                    skill_read_blocked: blocked,
                    tools_observed: tools,
                    elapsed_millis: start.elapsed().as_millis(),
                    tokens: used,
                    trace_sha256: digest(&raw),
                });
                fs::write(
                    output.join("result.json"),
                    serde_json::to_vec_pretty(&report)?,
                )?;
                println!(
                    "completed {id}: process={} answer={} tokens={used}",
                    status.success(),
                    value.is_some()
                );
                if !status.success() && value.is_none() {
                    // Infrastructure failures are not negative quality scores or
                    // permission to downgrade the sandbox. Stop repeated calls.
                    report.partial = true;
                    report.partial_reason = Some("runtime-infrastructure-failure".into());
                    break 'cases;
                }
                if blocked {
                    report.partial = true;
                    report.partial_reason = Some("skill-read-policy-blocked".into());
                    break 'cases;
                }
            }
        }
    }
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(!report.partial
        && report
            .rows
            .iter()
            .all(|r| r.process_ok && r.structured_answer))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn infrastructure_is_not_a_quality_failure() {
        assert!(!grade(&serde_json::json!({}), true, false));
    }
    #[test]
    fn native_units_do_not_establish_browser_qualification() {
        let value = serde_json::json!({"skill":"implement-sot-slice","plan":"Rust 1.99.1 edition 2024 cargo test --locked clippy native unit pass"});
        assert!(!grade(&value, true, true));
    }
    #[test]
    fn only_completed_commands_count() {
        let (read, tools, tokens) = observed(
            "{\"type\":\"item.started\",\"item\":{\"type\":\"command_execution\",\"command\":\"read implement-sot-slice/SKILL.md\"}}\n{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}",
        );
        assert!(read);
        assert_eq!(tools, 0);
        assert_eq!(tokens, 15);
    }
}
