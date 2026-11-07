use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_files: usize,
    pub max_roots: usize,
    pub max_total_read_bytes: u64,
    pub max_file_bytes: u64,
    pub max_elapsed: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_files: 5_000,
            max_roots: 64,
            max_total_read_bytes: 2 * 1024 * 1024 * 1024,
            max_file_bytes: 512 * 1024 * 1024,
            max_elapsed: Duration::from_secs(30),
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), SelectionError> {
        if self.max_files == 0
            || self.max_roots == 0
            || self.max_total_read_bytes == 0
            || self.max_elapsed.is_zero()
        {
            return Err(SelectionError::InvalidLimits);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Root {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub root: String,
    pub path: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectionError {
    InvalidLimits,
    EmptySelection,
    CountLimit,
    InvalidRoot,
    DuplicateRoot,
    InvalidPath,
    UnknownRoot,
    DuplicateInput,
}

/// Ownership freezes the selected set; this is not resolved dependency discovery.
pub struct FrozenSelection {
    pub(crate) roots: Vec<Root>,
    pub(crate) inputs: Vec<Input>,
    pub(crate) declared_complete: bool,
    digest: String,
}

impl FrozenSelection {
    pub fn new(
        roots: Vec<Root>,
        inputs: Vec<Input>,
        declared_complete: bool,
        limits: &Limits,
    ) -> Result<Self, SelectionError> {
        limits.validate()?;
        if roots.is_empty() || inputs.is_empty() {
            return Err(SelectionError::EmptySelection);
        }
        if roots.len() > limits.max_roots || inputs.len() > limits.max_files {
            return Err(SelectionError::CountLimit);
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for root in &roots {
            let path = root.path.to_str().ok_or(SelectionError::InvalidRoot)?;
            if root.id.is_empty()
                || root.id.len() > 64
                || !root
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || path.len() > 4096
                || !root.path.is_absolute()
                || root
                    .path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            {
                return Err(SelectionError::InvalidRoot);
            }
            if !ids.insert(root.id.clone()) || !paths.insert(path.to_lowercase()) {
                return Err(SelectionError::DuplicateRoot);
            }
        }
        let mut selected = BTreeSet::new();
        for input in &inputs {
            if !ids.contains(&input.root) {
                return Err(SelectionError::UnknownRoot);
            }
            if input.path.is_empty()
                || input.path.len() > 4096
                || input.path.contains(['\\', ':'])
                || input.path.chars().any(char::is_control)
                || input
                    .path
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == "..")
            {
                return Err(SelectionError::InvalidPath);
            }
            if !selected.insert((input.root.clone(), input.path.to_lowercase())) {
                return Err(SelectionError::DuplicateInput);
            }
        }
        let bytes = serde_json::to_vec(&(&roots, &inputs, declared_complete))
            .expect("strings and paths have a JSON representation");
        let digest = format!("{:x}", Sha256::digest(bytes));
        Ok(Self {
            roots,
            inputs,
            declared_complete,
            digest,
        })
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn len(&self) -> usize {
        self.inputs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}
