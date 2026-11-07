//! Actual legacy coordinator regression; all Git mutations stay in an owned fixture.
use std::{fs, path::PathBuf, process::Command};

struct Repository(PathBuf);

impl Repository {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-git-coord-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        repo.write("empty-git-config", "");
        fs::create_dir(repo.0.join("empty-hooks")).unwrap();
        repo.git(&["init", "--initial-branch=fixture"]);
        repo.git(&["config", "user.name", "codex"]);
        repo.git(&["config", "user.email", "codex@openai.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&[
            "config",
            "core.hooksPath",
            repo.0.join("empty-hooks").to_str().unwrap(),
        ]);
        repo
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.current_dir(&self.0);
        // A caller's worktree/index overrides must never redirect fixture writes.
        for (name, _) in std::env::vars_os() {
            if name
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_")
            {
                command.env_remove(name);
            }
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.0.join("empty-git-config"))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("TEMP", &self.0);
        command
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self.command("git").args(args).output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn single_literal_owned_path_commits_without_consuming_foreign_staging() {
    let repo = Repository::new();
    let owned = "owned [one].txt";
    repo.write(owned, "before\n");
    repo.write("foreign.txt", "before\n");
    repo.git(&["add", "--", owned, "foreign.txt"]);
    repo.git(&["commit", "-m", "test(fixture): seed isolated repository"]);
    repo.write(owned, "owned change\n");
    repo.write("foreign.txt", "foreign staged change\n");
    repo.git(&["add", "--", owned, "foreign.txt"]);
    repo.write(
        "scripts/agents/git-coord.ps1",
        include_str!("../../../scripts/agents/git-coord.ps1"),
    );
    let claim = serde_json::json!({
        "lane":"git-ops", "provider":"codex", "session":"fixture-owner",
        "goal":"Isolated single-path regression", "paths":"owned/**",
        "since":time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339).unwrap(),
        "status":"active"
    });
    repo.write(".agents/sessions/claims/git-ops.json", &claim.to_string());
    let output = repo
        .command("pwsh")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(repo.0.join("scripts/agents/git-coord.ps1"))
        .args([
            "commit",
            "-Provider",
            "codex",
            "-Subject",
            "fix(fixture): commit one literal owned file",
            "-OnlyPaths",
            owned,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "coordinator failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        repo.git(&["show", "--format=", "--name-only", "HEAD"])
            .trim(),
        owned
    );
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]).trim(),
        "foreign.txt"
    );
    let message = repo.git(&["show", "-s", "--format=%B", "HEAD"]);
    assert!(message.contains("Co-Authored-By: codex <codex@openai.com>"));
    assert!(message.contains("X-Agent-Provider: codex"));
}
