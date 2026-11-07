//! Closed pre-semantic-test gate. Every write targets a newly created fixture.
use super::*;
use std::{
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

const PAYLOAD: &[u8] = b"audit-native-prerequisite-v1\n";
const PREFIX: &str = "AUDIT_NATIVE_PREREQUISITE_V1 ";
const FILES: [&str; 2] = [LOCK, "audit-0000000001.log"];

struct Failure {
    stage: &'static str,
    kind: io::ErrorKind,
}
impl Failure {
    fn at(stage: &'static str, error: io::Error) -> Self {
        Self {
            stage,
            kind: error.kind(),
        }
    }
    fn status(&self) -> &'static str {
        match self.kind {
            io::ErrorKind::InvalidInput
            | io::ErrorKind::InvalidData
            | io::ErrorKind::Unsupported
            | io::ErrorKind::PermissionDenied
            | io::ErrorKind::NotFound
            | io::ErrorKind::AlreadyExists
            | io::ErrorKind::WouldBlock
            | io::ErrorKind::FileTooLarge => "refused",
            _ => "unknown",
        }
    }
}
struct Fixture {
    parent: Pin,
    name: String,
    path: PathBuf,
    root: Option<File>,
    created: [bool; 2],
}
impl Fixture {
    fn populate(&mut self) -> io::Result<()> {
        self.root = Some(relative(&self.parent.file, &self.name, true, false)?);
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| invalid("fixture root missing"))?;
        observe(root, Role::Directory)?;
        for (index, name) in FILES.iter().enumerate() {
            let fd = openat2(
                root,
                *name,
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NONBLOCK,
                Mode::RUSR | Mode::WUSR,
                ResolveFlags::BENEATH
                    | ResolveFlags::NO_SYMLINKS
                    | ResolveFlags::NO_MAGICLINKS
                    | ResolveFlags::NO_XDEV,
            )
            .map_err(api_error)?;
            self.created[index] = true;
            let mut file = File::from(fd);
            if index == 1 {
                file.write_all(PAYLOAD)?;
            }
            file.sync_all()?;
            observe(&file, Role::Payload)?;
        }
        root.sync_all()
    }

    fn exercise(&self, control: &Control<'_>) -> io::Result<()> {
        control.check()?;
        let mut profile = limits();
        profile.segments = 1;
        // Reuse the actual provider; do not fork identity/privacy/API checks.
        let source = NativeSource::open(
            &self.path,
            &profile,
            control,
            Admission {
                max_entries: 16,
                max_handles: 64,
                other_handles: 16,
            },
        )?;
        let mut first = source.segment(1)?;
        let mut second = source.segment(1)?;
        let mut prefix = [0; 3];
        first.read_exact(&mut prefix)?;
        let mut all = [0; PAYLOAD.len()];
        second.read_exact(&mut all)?;
        let mut suffix = [0; PAYLOAD.len() - 3];
        first.read_exact(&mut suffix)?;
        if prefix.as_slice() != &PAYLOAD[..3]
            || suffix.as_slice() != &PAYLOAD[3..]
            || all.as_slice() != PAYLOAD
        {
            return Err(invalid("fixture positioned read disagrees"));
        }
        let mut eof = [0];
        if first.read(&mut eof)? != 0 || second.read(&mut eof)? != 0 {
            return Err(invalid("fixture positioned EOF disagrees"));
        }
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| invalid("fixture root missing"))?;
        match ExclusiveFile::acquire(relative(root, LOCK, false, false)?) {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
            Err(error) => return Err(error),
            Ok(contender) => {
                drop(contender);
                return Err(invalid("fixture exclusive lock not enforced"));
            }
        }
        source.unchanged()?;
        drop(first);
        drop(second);
        drop(source);
        // Observe actual release, not merely the last Rust owner's drop intent.
        drop(ExclusiveFile::acquire(relative(root, LOCK, false, false)?)?);
        control.check()
    }

    fn cleanup(self) -> serde_json::Value {
        let mut issue = None;
        if let Some(root) = &self.root {
            for (index, name) in FILES.iter().enumerate() {
                if self.created[index]
                    && let Err(error) = rustix::fs::unlinkat(root, *name, AtFlags::empty())
                {
                    issue.get_or_insert(io::Error::from(error).kind());
                }
            }
            if let Err(error) = root.sync_all() {
                issue.get_or_insert(error.kind());
            }
        }
        // Only remove this mkdirat-created empty directory. Never recursively
        // delete unknown entries or clean a pre-existing caller directory.
        if let Err(error) =
            rustix::fs::unlinkat(&self.parent.file, self.name.as_str(), AtFlags::REMOVEDIR)
        {
            issue.get_or_insert(io::Error::from(error).kind());
        }
        if let Err(error) = self.parent.file.sync_all() {
            issue.get_or_insert(error.kind());
        }
        let absent = match relative(&self.parent.file, &self.name, true, false) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Some(true),
            Ok(_) => Some(false),
            Err(error) => {
                issue.get_or_insert(error.kind());
                None
            }
        };
        serde_json::json!({
            "state": if issue.is_none() && absent == Some(true) { "removed" } else { "unknown" },
            "name_absent": absent,
            "error_kind": issue.map(|kind| format!("{kind:?}")),
        })
    }
}

fn parent() -> Result<(Pin, PathBuf), Failure> {
    let path = std::env::var_os("HUMANYMOUS_AUDIT_NATIVE_SOURCE_PARENT")
        .map(PathBuf::from)
        .ok_or_else(|| Failure::at("parent-input", invalid("allocated parent required")))?;
    if !path.is_absolute()
        || path.as_os_str().as_bytes().len() > 4096
        || path
            .as_os_str()
            .as_bytes()
            .split(|byte| *byte == b'/')
            .any(|part| matches!(part, b"." | b".."))
    {
        return Err(Failure::at(
            "parent-input",
            invalid("unaliased absolute parent required"),
        ));
    }
    let file = absolute(&path).map_err(|error| Failure::at("parent-open", error))?;
    // The allocated parent itself must be current-owner0700 on the admitted
    // filesystem, not merely the fresh child produced by this gate.
    let pin = Pin::capture(file, Role::Directory)
        .map_err(|error| Failure::at("parent-provider", error))?;
    Ok((pin, path))
}

fn run() -> serde_json::Value {
    let uid = rustix::process::getuid().as_raw();
    let euid = rustix::process::geteuid().as_raw();
    let mut failure = None;
    let mut cleanup =
        serde_json::json!({"state":"not-created","name_absent":null,"error_kind":null});
    let observed = (|| -> Result<(), Failure> {
        if uid == 0 || euid == 0 || uid != euid {
            return Err(Failure::at(
                "nonroot-uid",
                invalid("ordinary matching UID required"),
            ));
        }
        let cancel = AtomicBool::new(false);
        let control = Control::new(&cancel, Duration::from_secs(30))
            .map_err(|error| Failure::at("operation-bounds", error))?;
        let (parent, path) = parent()?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Failure::at("fixture-name", invalid("fixture clock unavailable")))?
            .as_nanos();
        let name = format!("audit-prerequisite-{}-{nonce}", std::process::id());
        rustix::fs::mkdirat(
            &parent.file,
            name.as_str(),
            Mode::RUSR | Mode::WUSR | Mode::XUSR,
        )
        .map_err(|error| Failure::at("fixture-create", error.into()))?;
        let mut fixture = Fixture {
            parent,
            path: path.join(&name),
            name,
            root: None,
            created: [false; 2],
        };
        let result = fixture
            .populate()
            .map_err(|error| Failure::at("fixture-bootstrap", error))
            .and_then(|()| {
                fixture
                    .exercise(&control)
                    .map_err(|error| Failure::at("native-provider", error))
            });
        cleanup = fixture.cleanup();
        result
    })();
    if let Err(error) = observed {
        failure = Some(error);
    }
    let clean = cleanup["state"] == "removed";
    let status = if !clean && cleanup["state"] != "not-created" {
        "unknown"
    } else {
        failure.as_ref().map_or("observed", Failure::status)
    };
    serde_json::json!({
        "schema":1, "gate":"audit-native-filesystem-v1", "status":status,
        "uid":uid, "euid":euid, "cleanup":cleanup,
        "checks": if failure.is_none() && clean { serde_json::json!({
            "nonroot_uid":true,"held_directory_0700":true,"held_payload_0600":true,
            "ext_family_0xef53":true,"same_held_statx_mount_id":true,
            "openat2_no_symlinks_no_magiclinks":true,"read_at":true,
            "exclusive_lock":true,"last_drop_lock_release":true,
        }) } else { serde_json::Value::Null },
        "first_failure":failure.map(|error| serde_json::json!({
            "stage":error.stage,"error_kind":format!("{:?}",error.kind),
        })),
    })
}

#[test]
#[ignore = "closed CI prerequisite; requires explicitly allocated fresh private parent"]
fn native_filesystem_prerequisite() {
    let receipt = run();
    // Stderr keeps the named libtest stdout footer intact under --nocapture.
    eprintln!("{PREFIX}{receipt}");
    assert!(
        receipt["status"] == "observed" && receipt["cleanup"]["state"] == "removed",
        "native filesystem prerequisite not satisfied; inspect fixed receipt"
    );
}
