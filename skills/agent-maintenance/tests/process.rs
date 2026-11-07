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
