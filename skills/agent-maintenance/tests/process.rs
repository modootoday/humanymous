//! Actual executable boundary tests using temporary owned fixtures.
use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "humanymous-agent-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn write(&self, path: &str, text: &str) {
        let p = self.0.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    fn canon(&self) {
        self.write(
            "AGENTS.md",
            "1.99.1 2024 Detection freeze Fail-closed dual-control A11y T4 Docker git-ops\n",
        );
        self.write(
            ".agents/skills/example/SKILL.md",
            "---\nname: example\ndescription: Use when example\n---\n",
        );
        self.write(".agents/rules/example.md", "Rust only\n");
        for name in [
            "blue-architect",
            "red-attacker",
            "evaluator",
            "staff-synthesizer",
            "sre-ops",
            "compliance-dpo",
        ] {
            self.write(&format!(".agents/agents/{name}.md"), "Canonical role\n");
            self.write(
                &format!(".agents/personas/{name}.md"),
                &format!("../agents/{name}.md\n"),
            );
        }
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_humanymous-agent-maintenance"))
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn session_fixture() -> Fixture {
    let f = Fixture::new();
    f.write(
        ".agents/sessions/lanes.json",
        r#"{"schema_version":1,"writer_mode":"rust-exclusive","lanes":[{"id":"one","kind":"shared","primary_paths":[]}]}"#,
    );
    fs::create_dir_all(f.0.join(".agents/sessions/claims")).unwrap();
    f.write(
        ".agents/sessions/ACTIVE.md",
        "# Board\n\n## Handovers\n\nRetained handover\n",
    );
    f
}

fn request(f: &Fixture, session: &str) -> PathBuf {
    let name = format!("{session}.json");
    f.write(
        &name,
        &serde_json::json!({
            "lane":"one", "provider":"codex", "session":session,
            "goal":"Process ownership fixture", "paths":"owned/**"
        })
        .to_string(),
    );
    f.0.join(name)
}

#[test]
fn competing_claim_processes_leave_exactly_one_owner() {
    let f = session_fixture();
    let requests = [request(&f, "first"), request(&f, "second")];
    let children: Vec<_> = requests
        .iter()
        .map(|path| {
            Command::new(env!("CARGO_BIN_EXE_humanymous-agent-maintenance"))
                .arg("session-claim")
                .arg(&f.0)
                .arg(path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    // Both children are launched before either is waited; scheduling overlap is not assumed.
    let results: Vec<_> = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|r| r.status.success()).count(), 1);
    let winner = results.iter().position(|r| r.status.success()).unwrap();
    let sessions = f.0.join(".agents/sessions");
    let claim: serde_json::Value =
        serde_json::from_slice(&fs::read(sessions.join("claims/one.json")).unwrap()).unwrap();
    assert_eq!(claim["provider"], "codex");
    assert_eq!(claim["session"], ["first", "second"][winner]);
    assert_eq!(fs::read_dir(sessions.join("claims")).unwrap().count(), 1);
    assert!(!sessions.join(".session-tool.lock").exists());
    let events = fs::read_to_string(sessions.join("events.jsonl")).unwrap();
    let actions: Vec<String> = events
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            value["action"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(actions, ["claim-intent", "claimed"]);
}

#[test]
fn legacy_bom_claim_keeps_owner_until_exact_owner_releases() {
    let f = session_fixture();
    let original = concat!(
        "\u{feff}",
        r#"{"lane":"one","provider":"codex","session":"legacy-owner","goal":"Retained legacy claim","paths":"owned/**","since":"2000-01-01T00:00:00Z","status":"active"}"#
    );
    f.write(".agents/sessions/claims/one.json", original);
    let root = f.0.to_str().unwrap();
    let listed = f.run(&["session-list", root]);
    assert!(listed.status.success());
    let claims: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(claims["claims"][0]["session"], "legacy-owner");
    let next = request(&f, "new-owner");
    assert!(
        !f.run(&["session-claim", root, next.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        !f.run(&[
            "session-release",
            root,
            "one",
            "codex",
            "new-owner",
            "Denied"
        ])
        .status
        .success()
    );
    let sessions = f.0.join(".agents/sessions");
    assert_eq!(
        fs::read(sessions.join("claims/one.json")).unwrap(),
        original.as_bytes()
    );
    assert!(!sessions.join("events.jsonl").exists());
    assert!(!sessions.join(".session-tool.lock").exists());
    assert!(
        f.run(&[
            "session-release",
            root,
            "one",
            "codex",
            "legacy-owner",
            "Owned release",
        ])
        .status
        .success()
    );
    assert!(!sessions.join("claims/one.json").exists());
    let board = fs::read_to_string(sessions.join("ACTIVE.md")).unwrap();
    assert!(board.contains("Retained handover"));
    assert!(board.contains("Owned release"));
}

#[test]
fn canonical_roles_do_not_require_retired_persona_pointers() {
    let f = Fixture::new();
    f.canon();
    fs::remove_dir_all(f.0.join(".agents/personas")).unwrap();
    let output = f.run(&["check", f.0.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["canonical_roles"], 6);
    assert_eq!(result["role_pointers"], 0);
}

#[test]
fn checks_real_tree_and_rejects_wrong_skill_name() {
    let f = Fixture::new();
    f.canon();
    let output = f.run(&["check", f.0.to_str().unwrap()]);
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["role_pointers"], 6);
    f.write(
        ".agents/skills/example/SKILL.md",
        "---\nname: wrong\ndescription: Use when x\n---\n",
    );
    assert!(!f.run(&["check", f.0.to_str().unwrap()]).status.success());
}
#[test]
fn normalize_keeps_mirrors_and_preserves_content() {
    let f = Fixture::new();
    f.canon();
    f.write(".agents/rules/example.md", "a\r\nb\r\n");
    f.write(".codex/skills/keep.md", "old\r\n");
    assert!(!f.run(&["check", f.0.to_str().unwrap()]).status.success());
    assert!(
        f.run(&["normalize", f.0.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(
        fs::read(f.0.join(".agents/rules/example.md")).unwrap(),
        b"a\nb\n"
    );
    assert_eq!(
        fs::read(f.0.join(".codex/skills/keep.md")).unwrap(),
        b"old\r\n"
    );
}
#[test]
fn missing_routing_rule_is_an_error_not_a_negative_match() {
    let f = Fixture::new();
    f.write(".agents/evals/trigger-rules.json", r#"{"rules":{}}"#);
    f.write(".agents/evals/trigger-queries.json",r#"{"queries":[{"query":"x","skill":"missing","should_trigger":false,"split":"validation"}]}"#);
    assert!(!f.run(&["routing", f.0.to_str().unwrap()]).status.success());
}
#[test]
fn eval_never_overwrites_existing_output() {
    let f = Fixture::new();
    f.write("keep.txt", "retained");
    assert!(
        !f.run(&["eval", ".", "not-a-cli", f.0.to_str().unwrap(), "test"])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(f.0.join("keep.txt")).unwrap(),
        "retained"
    );
}
