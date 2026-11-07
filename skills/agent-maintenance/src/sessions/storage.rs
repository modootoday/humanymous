//! Rust-only session storage adapter for an explicitly exclusive writer rollout.
//! Trusted repository directory only; this is not an OS authorization boundary.
use super::{
    catalog::{Catalog, WriterMode},
    model::{Claim, Request, admit},
};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Serialize)]
pub struct Snapshot {
    pub schema: &'static str,
    pub writer_mode: WriterMode,
    pub claims: Vec<Claim>,
    pub runtime_qualified: bool,
}

fn read_text(path: &Path, limit: u64) -> Result<String, Box<dyn std::error::Error>> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > limit {
        return Err(format!("unsafe/oversized session file: {}", path.display()).into());
    }
    Ok(fs::read_to_string(path)?)
}
fn directory(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err("session directory/link refused".into());
    }
    Ok(())
}
fn layout(root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    directory(root)?;
    directory(&root.join(".agents"))?;
    let base = root.join(".agents/sessions");
    directory(&base)?;
    directory(&base.join("claims"))?;
    Ok(base)
}
fn catalog(base: &Path) -> Result<Catalog, Box<dyn std::error::Error>> {
    Ok(Catalog::parse(&read_text(
        &base.join("lanes.json"),
        256 * 1024,
    )?)?)
}
fn claims(base: &Path, catalog: &Catalog) -> Result<Vec<Claim>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(base.join("claims"))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".json") {
            return Err(format!("unknown claim entry: {name}").into());
        }
        let value = Claim::parse(&read_text(&entry.path(), 64 * 1024)?, catalog)?;
        if name != format!("{}.json", value.lane) {
            return Err("claim filename/lane mismatch".into());
        }
        result.push(value);
        if result.len() > 128 {
            return Err("claim count exceeds128".into());
        }
    }
    result.sort_by(|a, b| a.lane.cmp(&b.lane));
    Ok(result)
}

pub fn list(root: &Path) -> Result<Snapshot, Box<dyn std::error::Error>> {
    let base = layout(root)?;
    let catalog = catalog(&base)?;
    let claims = claims(&base, &catalog)?;
    Ok(Snapshot {
        schema: "humanymous.sessions.v1",
        writer_mode: catalog.writer_mode,
        claims,
        runtime_qualified: false,
    })
}

struct Lock {
    path: PathBuf,
    file: Option<File>,
    retain: bool,
}
impl Lock {
    fn mutation_started(&mut self) {
        self.retain = true;
    }
    fn finish(mut self) -> Result<(), std::io::Error> {
        self.file.take();
        fs::remove_file(&self.path)?;
        self.retain = false;
        Ok(())
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        self.file.take();
        if !self.retain {
            let _ = fs::remove_file(&self.path);
        }
    }
}
fn lock(base: &Path, catalog: &Catalog) -> Result<Lock, Box<dyn std::error::Error>> {
    if catalog.writer_mode != WriterMode::RustExclusive {
        return Err("mutations refused during legacy coexistence; complete coordinated exclusive-writer rollout first".into());
    }
    let path = base.join(".session-tool.lock");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(b"humanymous.sessions.v1\n")?;
    file.sync_all()?;
    Ok(Lock {
        path,
        file: Some(file),
        retain: false,
    })
}

fn validate_note(note: &str) -> Result<(), &'static str> {
    if note.len() > 4096 || note.chars().any(char::is_control) {
        return Err("invalid handover note");
    }
    Ok(())
}

fn event(
    base: &Path,
    action: &str,
    claim: &Claim,
    note: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_note(note)?;
    let path = base.join("events.jsonl");
    if path.exists() {
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > 16 * 1024 * 1024 {
            return Err("unsafe/full session event journal".into());
        }
    }
    let data = serde_json::to_vec(
        &serde_json::json!({"schema":"humanymous.session-event.v1","at":OffsetDateTime::now_utc().format(&Rfc3339)?,"action":action,"claim":claim,"note":note}),
    )?;
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(&data)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

/// Preserve the existing handover tail while rendering a compatible claims table.
fn board(
    base: &Path,
    catalog: &Catalog,
    handover: Option<(&Claim, &str)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("ACTIVE.md");
    let old = if path.exists() {
        read_text(&path, 2 * 1024 * 1024)?
    } else {
        String::new()
    };
    let tail = old
        .find("## Handovers")
        .map(|i| old[i..].to_owned())
        .unwrap_or_else(|| "## Handovers (incomplete work)\n".into());
    let mut text = String::from(
        "# ACTIVE session board\n\nGenerated by Rust session tools; handover tail preserved.\n\n## Claims\n\n| Lane | Provider | Session | Goal | Paths | Since | Status |\n|---|---|---|---|---|---|---|\n",
    );
    let escape = |s: &str| s.replace('|', "\\|");
    for c in claims(base, catalog)? {
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            c.lane,
            c.provider,
            escape(&c.session),
            escape(&c.goal),
            escape(&c.paths),
            c.since,
            c.status
        ));
    }
    text.push('\n');
    text.push_str(&tail);
    if let Some((claim, note)) = handover {
        text.push_str(&format!(
            "\n### {} — release {}\n\n- **Note:** {}\n- **Was:** {}/{}: {}\n",
            OffsetDateTime::now_utc().format(&Rfc3339)?,
            claim.lane,
            escape(note),
            claim.provider,
            escape(&claim.session),
            escape(&claim.goal)
        ));
    }
    let temporary = base.join(".ACTIVE.rust-next");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}

/// Acquire only after all shared writers have explicitly switched to this protocol.
pub fn claim(root: &Path, mut request: Claim) -> Result<Claim, Box<dyn std::error::Error>> {
    let base = layout(root)?;
    let catalog = catalog(&base)?;
    let mut lock = lock(&base, &catalog)?;
    request.since = OffsetDateTime::now_utc().format(&Rfc3339)?;
    request.status = "active".into();
    admit(&request, &claims(&base, &catalog)?, &catalog)?;
    if request.lane == "git-ops"
        && (!root.join(".git").is_dir() || root.join(".git/index.lock").exists())
    {
        return Err("git-ops requires a resolved native Git directory without index.lock; linked worktree is unsupported here".into());
    }
    lock.mutation_started();
    event(&base, "claim-intent", &request, "")?;
    let path = base.join("claims").join(format!("{}.json", request.lane));
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&serde_json::to_vec_pretty(&request)?)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    // Post-write errors keep the claim/evidence fail-closed, never silently release it.
    event(&base, "claimed", &request, "")?;
    board(&base, &catalog, None)?;
    lock.finish()?;
    Ok(request)
}

pub fn claim_request_file(root: &Path, path: &Path) -> Result<Claim, Box<dyn std::error::Error>> {
    let request = Request::parse(&read_text(path, 32 * 1024)?)?;
    claim(
        root,
        request.into_claim(OffsetDateTime::now_utc().format(&Rfc3339)?),
    )
}

pub fn release(
    root: &Path,
    lane: &str,
    provider: &str,
    session: &str,
    note: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Invalid input must not create a retained mutation lock.
    validate_note(note)?;
    let base = layout(root)?;
    let catalog = catalog(&base)?;
    if !catalog.contains(lane) {
        return Err("unknown lane".into());
    }
    let mut lock = lock(&base, &catalog)?;
    let path = base.join("claims").join(format!("{lane}.json"));
    let claim = Claim::parse(&read_text(&path, 64 * 1024)?, &catalog)?;
    if !claim.owned_by(provider, session) {
        return Err("release denied: different claim owner".into());
    }
    // Intent is durable before removing the claim. No force or automatic stale takeover.
    lock.mutation_started();
    event(&base, "release-intent", &claim, note)?;
    fs::remove_file(path)?;
    event(&base, "released", &claim, note)?;
    board(&base, &catalog, Some((&claim, note)))?;
    lock.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_note;

    #[test]
    fn handover_input_is_validated_before_mutation() {
        assert!(validate_note("Owned source checkpoint; hosted checks pending").is_ok());
        assert!(validate_note("Unicode names: café · 서울").is_ok());
        assert!(validate_note("forged\nboard row").is_err());
        assert!(validate_note("nul\0byte").is_err());
        assert!(validate_note(&"x".repeat(4097)).is_err());
    }
}
