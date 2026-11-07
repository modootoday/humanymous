use super::{
    Clock, ContentRead, FrozenSelection, Input, Limits, ReadBudget, ReadStatus, Root,
    SelectionError, SystemClock,
};
use serde::Serialize;
use std::{
    fs::{self, File, Metadata},
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    ObservedStable,
    Changed,
    Unknown,
}

#[derive(Serialize)]
pub struct FileObservation {
    pub input: Input,
    pub content: ContentRead,
    pub stability: Stability,
    pub provider_identity_qualified: bool,
}

#[derive(Serialize)]
pub struct PreparationReceipt {
    pub schema: &'static str,
    pub scope: &'static str,
    pub selection_sha256: String,
    pub input_selection_complete: bool,
    pub content_read_complete: bool,
    pub provider_identity_qualified: bool,
    pub qualified_complete: bool,
    pub actual_read_bytes: u64,
    pub read_count_known: bool,
    pub probe_bytes: u64,
    pub elapsed_micros: u128,
    pub files: Vec<FileObservation>,
    pub errors: Vec<String>,
    pub immutable_snapshot: bool,
    pub execution_authorized: bool,
    pub cache_authorized: bool,
}

#[derive(PartialEq, Eq)]
struct Stamp {
    bytes: u64,
    modified: SystemTime,
}
fn stamp(metadata: &Metadata) -> io::Result<Stamp> {
    Ok(Stamp {
        bytes: metadata.len(),
        modified: metadata.modified()?,
    })
}

fn linked(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// Check each selected ancestor without walking directory entries.
/// Concurrent hostile replacement is deliberately not an identity guarantee.
pub(super) fn checked_path(path: &Path, file: bool) -> io::Result<Metadata> {
    let mut selected = PathBuf::new();
    for component in path.components() {
        selected.push(component.as_os_str());
        if !selected.has_root() {
            continue;
        }
        let metadata = fs::symlink_metadata(&selected)?;
        if linked(&metadata) || (selected != path && !metadata.is_dir()) {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
    }
    let metadata = fs::symlink_metadata(path)?;
    if linked(&metadata) || (file && !metadata.is_file()) || (!file && !metadata.is_dir()) {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    Ok(metadata)
}

struct Opened {
    file: File,
    path: PathBuf,
    before: Stamp,
}
impl Opened {
    fn open(root: &Root, input: &Input) -> io::Result<Self> {
        checked_path(&root.path, false)?;
        let path = root.path.join(&input.path);
        let before_path = stamp(&checked_path(&path, true)?)?;
        let file = File::open(&path)?;
        let opened_metadata = file.metadata()?;
        if !opened_metadata.is_file() || stamp(&opened_metadata)? != before_path {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        Ok(Self {
            file,
            path,
            before: before_path,
        })
    }
    fn stability(&self) -> io::Result<Stability> {
        let handle = stamp(&self.file.metadata()?)?;
        let path = stamp(&checked_path(&self.path, true)?)?;
        Ok(if handle == self.before && path == self.before {
            Stability::ObservedStable
        } else {
            Stability::Changed
        })
    }
}

fn observe_one<C: Clock>(
    root: &Root,
    input: &Input,
    budget: &mut ReadBudget,
    clock: &mut C,
) -> FileObservation {
    let mut observation = FileObservation {
        input: input.clone(),
        content: ContentRead::refused(ReadStatus::ProviderRefused, "unopened"),
        stability: Stability::Unknown,
        provider_identity_qualified: false,
    };
    if let Err(status) = budget.check(clock) {
        observation.content = ContentRead::refused(status, "cooperative_budget");
        return observation;
    }
    let mut opened = match Opened::open(root, input) {
        Ok(opened) => opened,
        Err(error) => {
            observation.content.error_kind = Some(format!("{:?}", error.kind()));
            return observation;
        }
    };
    observation.content = budget.read(&mut opened.file, clock);
    observation.stability = opened.stability().unwrap_or(Stability::Unknown);
    if observation.stability != Stability::ObservedStable && observation.content.complete() {
        observation.content.status = if observation.stability == Stability::Changed {
            ReadStatus::Changed
        } else {
            ReadStatus::ProviderUnknown
        };
        observation.content.sha256 = None;
    }
    observation
}

/// Observe only the frozen selection. Metadata equality is not immutable identity.
/// The provider remains unqualified; the receipt never authorizes cache/execution.
pub fn observe(
    selection: &FrozenSelection,
    limits: Limits,
) -> Result<PreparationReceipt, SelectionError> {
    observe_with_clock(selection, limits, &mut SystemClock::default())
}

pub(super) fn observe_with_clock<C: Clock>(
    selection: &FrozenSelection,
    limits: Limits,
    clock: &mut C,
) -> Result<PreparationReceipt, SelectionError> {
    limits.validate()?;
    if selection.roots.len() > limits.max_roots || selection.inputs.len() > limits.max_files {
        return Err(SelectionError::CountLimit);
    }
    let mut budget = ReadBudget::new(limits)?;
    let mut files = Vec::with_capacity(selection.inputs.len());
    for input in &selection.inputs {
        let root = selection
            .roots
            .iter()
            .find(|r| r.id == input.root)
            .expect("frozen selection validates root membership");
        files.push(observe_one(root, input, &mut budget, clock));
    }
    let mut errors: Vec<_> = files
        .iter()
        .filter(|f| !f.content.complete())
        .map(|f| format!("{}/{}: {:?}", f.input.root, f.input.path, f.content.status))
        .collect();
    if let Err(status) = budget.check(clock) {
        errors.push(format!("final_budget: {status:?}"));
    }
    let content_read_complete = errors.is_empty() && files.iter().all(|f| f.content.complete());
    Ok(PreparationReceipt {
        schema: "humanymous.selected-input-observation.v1",
        scope: "trusted-repository cooperative content observation; selected set only",
        selection_sha256: selection.digest().into(),
        input_selection_complete: selection.declared_complete,
        content_read_complete,
        provider_identity_qualified: false,
        qualified_complete: false,
        actual_read_bytes: budget.actual_read_bytes(),
        read_count_known: budget.read_count_known(),
        probe_bytes: budget.probe_bytes(),
        elapsed_micros: budget.elapsed().as_micros(),
        files,
        errors,
        immutable_snapshot: false,
        execution_authorized: false,
        cache_authorized: false,
    })
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn command_clock_is_not_restarted_for_selected_observation() {
        struct Expired;
        impl Clock for Expired {
            fn elapsed(&mut self) -> std::time::Duration {
                std::time::Duration::from_secs(30)
            }
        }
        let selection = FrozenSelection::new(
            vec![Root {
                id: "r".into(),
                path: std::env::temp_dir(),
            }],
            vec![Input {
                root: "r".into(),
                path: "not-read-after-deadline".into(),
            }],
            true,
            &Limits::default(),
        )
        .unwrap();
        let receipt = observe_with_clock(&selection, Limits::default(), &mut Expired).unwrap();
        assert!(!receipt.content_read_complete);
        assert_eq!(receipt.actual_read_bytes, 0);
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.contains("Deadline"))
        );
    }

    #[test]
    fn changed_open_file_is_not_observed_stable() {
        let directory = std::env::temp_dir().join(format!(
            "hmn-prep-change-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("input");
        fs::write(&path, b"before").unwrap();
        let opened = Opened::open(
            &Root {
                id: "r".into(),
                path: directory.clone(),
            },
            &Input {
                root: "r".into(),
                path: "input".into(),
            },
        )
        .unwrap();
        File::options()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"changed")
            .unwrap();
        assert_eq!(opened.stability().unwrap(), Stability::Changed);
        drop(opened);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
