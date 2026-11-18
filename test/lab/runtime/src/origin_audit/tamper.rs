//! Disk tampering actor. It mounts only the WAL volume, never the sealed
//! custody, so it holds no HMAC or signing key. It copies the live history to
//! `case-<mode>` and mutates only the copy; the live history is never changed.
//!
//! - `copy`: unchanged copy (control for the copy procedure itself).
//! - `torn`: a partial, newline-less record appended (crash mid-write).
//! - `forge`: an ALLOW decision record rewritten to DENY, still valid JSON.
//! - `truncate`: the last record removed; its checkpoint kept.
//! - `rollback`: the last record and its checkpoint removed (a consistent
//!   prefix that only the independently retained trusted head reveals).
use super::{LIVE, TAMPER_ACCEPTED, TAMPER_REFUSED, WAL_ROOT, emit, history_name};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufReader},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const CHECKPOINTS: &str = "checkpoints.jsonl";
const LOCK: &str = ".audit-writer.lock";

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_owned())
}

fn lines(path: &Path) -> io::Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    if !text.is_empty() && !text.ends_with('\n') {
        return Err(invalid("source history has an uncommitted tail"));
    }
    Ok(text.lines().map(str::to_owned).collect())
}

fn write_lines(path: &Path, lines: &[String]) -> io::Result<()> {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    fs::write(path, text)
}

fn last_segment(directory: &Path) -> io::Result<PathBuf> {
    let mut segments: Vec<_> = fs::read_dir(directory)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("audit-") && name.ends_with(".log"))
        })
        .collect();
    segments.sort();
    segments.pop().ok_or_else(|| invalid("no audit segment"))
}

/// Copy `root/live` to `root/case-<mode>` and apply `mode` to the copy.
pub fn prepare(root: &Path, mode: &str) -> io::Result<Value> {
    if !TAMPER_ACCEPTED.contains(&mode) && !TAMPER_REFUSED.contains(&mode) {
        return Err(invalid("unknown tamper mode"));
    }
    let name = format!("case-{mode}");
    if !history_name(&name) {
        return Err(invalid("tamper target name"));
    }
    let source = root.join(LIVE);
    let target = root.join(&name);
    fs::create_dir(&target)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() || entry.file_name() == LOCK {
            continue;
        }
        let destination = target.join(entry.file_name());
        fs::copy(entry.path(), &destination)?;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o600))?;
    }
    let segment = last_segment(&target)?;
    let mut records = lines(&segment)?;
    let checkpoints_path = target.join(CHECKPOINTS);
    let mut checkpoints = lines(&checkpoints_path)?;
    let before = (records.len(), checkpoints.len());
    if records.is_empty() || checkpoints.is_empty() {
        return Err(invalid("live history must hold records and checkpoints"));
    }
    let change = match mode {
        "copy" => json!("none"),
        "torn" => {
            let mut bytes = fs::read(&segment)?;
            bytes.extend_from_slice(b"{\"seq\":");
            fs::write(&segment, bytes)?;
            json!("partial record without newline appended")
        }
        "forge" => {
            let position = records
                .iter()
                .rposition(|line| {
                    serde_json::from_str::<Value>(line)
                        .is_ok_and(|record| record["enforcement_action"] == "ALLOW")
                })
                .ok_or_else(|| invalid("no ALLOW decision to forge"))?;
            let mut record: Value = serde_json::from_str(&records[position])?;
            record["verdict"] = json!("DENY");
            record["enforcement_action"] = json!("DENY");
            records[position] = serde_json::to_string(&record)?;
            write_lines(&segment, &records)?;
            json!({"rewritten_seq":record["seq"],"from":"ALLOW","to":"DENY"})
        }
        "truncate" => {
            records.pop();
            write_lines(&segment, &records)?;
            json!("last record removed, checkpoints kept")
        }
        "rollback" => {
            records.pop();
            checkpoints.pop();
            write_lines(&segment, &records)?;
            write_lines(&checkpoints_path, &checkpoints)?;
            json!("last record and its checkpoint removed")
        }
        _ => return Err(invalid("unknown tamper mode")),
    };
    Ok(
        json!({"role":"oa_tamper","mode":mode,"target":name,"records_before":before.0,
        "checkpoints_before":before.1,"change":change,"live_modified":false}),
    )
}

/// One control line on stdin: `{"mode":MODE}`.
pub fn run() {
    let result = (|| -> io::Result<Value> {
        let control = super::read_line(&mut BufReader::new(io::stdin()), 1024)?;
        let mode = control["mode"]
            .as_str()
            .ok_or_else(|| invalid("tamper mode"))?;
        prepare(Path::new(WAL_ROOT), mode)
    })();
    let (value, ok) = match result {
        Ok(value) => (value, true),
        Err(error) => (
            json!({"role":"oa_tamper","error":error.to_string(),"live_modified":false}),
            false,
        ),
    };
    emit(&value).expect("tamper report output");
    if !ok {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{KEYS, NODE, document};
    use super::*;
    use humanymous_runtime::{
        audit::{
            checkpoint::Checkpoint,
            keys::{
                KeyMaterial,
                custody::{Custody, Owner},
            },
            log::{SealedLog, SealedLogOptions, UnlockCredential},
            record::Record,
        },
        gateway::{configuration::Configuration, startup::open_audit},
    };
    use std::os::unix::fs::MetadataExt;
    use zeroize::Zeroizing;

    const PASSPHRASE: &str = "synthetic-origin-audit-test";

    struct Fixture {
        root: PathBuf,
        keys: PathBuf,
        uid: u32,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn private(path: &Path) {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    /// Real sealed custody and a live history of three ALLOW decisions written
    /// through the runtime's own sealed log (checkpoint cadence one).
    fn fixture() -> (Fixture, Checkpoint) {
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce).unwrap();
        let name: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let root = std::env::temp_dir().join(format!("origin-audit-test-{name}"));
        private(&root);
        let root = root.canonicalize().unwrap();
        let keys = root.join("keys");
        private(&keys);
        let wal = root.join("wal");
        private(&wal);
        private(&wal.join(LIVE));
        let uid = fs::metadata(&root).unwrap().uid();
        let mut seed = [0; 32];
        let mut hmac = [0; 32];
        getrandom::fill(&mut seed).unwrap();
        getrandom::fill(&mut hmac).unwrap();
        let mut custody = Custody::acquire(&keys, Owner::CurrentUser).unwrap();
        custody
            .load_or_create(PASSPHRASE.as_bytes(), || {
                KeyMaterial::new(seed, hmac.to_vec(), None, None)
            })
            .unwrap();
        drop(custody);
        let options = SealedLogOptions {
            key_directory: keys.clone(),
            wal_directory: wal.join(LIVE),
            owner: Owner::CurrentUser,
            node_id: NODE.into(),
            checkpoint_every: 1,
            allow_new_history: true,
            trusted_head: None,
            require_external_witness: false,
        };
        let mut unlock = Zeroizing::new(PASSPHRASE.to_owned());
        let mut log = SealedLog::open(&options, UnlockCredential::Utf8(&mut unlock), None).unwrap();
        for _ in 0..3 {
            log.append(Record {
                event_type: "runtime.decision".into(),
                verdict: "ALLOW".into(),
                enforcement_action: "ALLOW".into(),
                ..Default::default()
            })
            .unwrap();
        }
        log.flush().unwrap();
        let head = log.checkpoints().unwrap().pop().unwrap();
        assert_eq!(head.tree_size, 3);
        assert_eq!(head.sig.len(), 128, "writer-signed checkpoint");
        drop(log);
        (Fixture { root, keys, uid }, head)
    }

    /// The exact startup authentication the gateway binary runs.
    fn startup(fixture: &Fixture, history: &str, head: Option<&Checkpoint>) -> Result<(), String> {
        let audit = fixture.root.join("wal").join(history);
        let document = document(
            audit.to_str().unwrap(),
            fixture.keys.to_str().unwrap(),
            PASSPHRASE,
            false,
            serde_json::to_value(head).unwrap(),
            fixture.uid,
        );
        let mut config: Configuration = serde_json::from_value(document).unwrap();
        open_audit(&mut config)
            .map(drop)
            .map_err(|error| error.to_owned())
    }

    #[test]
    fn every_mutation_has_the_runtime_startup_outcome_the_lab_asserts() {
        let (fixture, head) = fixture();
        let wal = fixture.root.join("wal");
        assert_eq!(startup(&fixture, LIVE, Some(&head)), Ok(()));
        for mode in TAMPER_ACCEPTED {
            let report = prepare(&wal, mode).unwrap();
            assert_eq!(report["records_before"], 3);
            assert_eq!(
                startup(&fixture, &format!("case-{mode}"), Some(&head)),
                Ok(()),
                "{mode}"
            );
        }
        for mode in TAMPER_REFUSED {
            prepare(&wal, mode).unwrap();
            assert_eq!(
                startup(&fixture, &format!("case-{mode}"), Some(&head)),
                Err("audit authenticated recovery".to_owned()),
                "{mode}"
            );
        }
        // The live history is never touched by any mutation.
        assert_eq!(startup(&fixture, LIVE, Some(&head)), Ok(()));
        assert_eq!(
            lines(&last_segment(&wal.join(LIVE)).unwrap())
                .unwrap()
                .len(),
            3
        );
    }

    /// Go parity note: without an independently retained head a consistent
    /// rollback prefix still authenticates; the anchor is what refuses it.
    #[test]
    fn rollback_is_refused_only_by_the_trusted_head() {
        let (fixture, head) = fixture();
        prepare(&fixture.root.join("wal"), "rollback").unwrap();
        assert_eq!(startup(&fixture, "case-rollback", None), Ok(()));
        assert!(startup(&fixture, "case-rollback", Some(&head)).is_err());
    }

    #[test]
    fn unknown_modes_existing_targets_and_empty_histories_are_refused() {
        let (fixture, _) = fixture();
        let wal = fixture.root.join("wal");
        assert!(prepare(&wal, "delete").is_err());
        assert!(prepare(&wal, "../live").is_err());
        prepare(&wal, "copy").unwrap();
        assert!(prepare(&wal, "copy").is_err(), "existing target reused");
        let empty = fixture.root.join("empty");
        private(&empty);
        private(&empty.join(LIVE));
        assert!(prepare(&empty, "forge").is_err());
        assert!(Path::new(KEYS).is_absolute());
    }
}
