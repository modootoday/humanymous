//! Trusted-checkout drift detection, not native custody or hostile-race proof.
use crate::docker::Result;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs::{self, File, Metadata},
    io::Read,
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};

const MAX_BYTES: usize = 16 * 1024;
static PROFILE: OnceLock<Value> = OnceLock::new();

fn profile() -> &'static Value {
    PROFILE.get_or_init(|| {
        humanymous_platform::testing::json::decode_strict_json(
            include_bytes!("../collector-runtime-pilot-source.json"),
            MAX_BYTES,
        )
        .expect("owned source pin DATA")
    })
}

/// Captured from the same bytes freeze copies, never loaded from a manifest.
#[derive(Clone)]
pub(crate) struct SelectedSource {
    path: String,
    sha256: String,
    bytes: usize,
}

pub(crate) fn selected(relative: &str, bytes: &[u8]) -> Option<SelectedSource> {
    (profile()["path"].as_str() == Some(relative)).then(|| SelectedSource {
        path: relative.into(),
        sha256: super::digest(bytes),
        bytes: bytes.len(),
    })
}

/// Explicit source identity and build-root join; not an authenticated capability.
pub(crate) struct SourceBinding {
    root: PathBuf,
    selected: SelectedSource,
    profile: &'static Value,
    compose_joined: bool,
}

fn regular(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    true
}

fn read(root: &Path, relative: &str) -> Result<Vec<u8>> {
    if !root.is_absolute()
        || root.canonicalize()? != root
        || !regular(&fs::symlink_metadata(root)?)
    {
        return Err("explicit canonical source root required".into());
    }
    let mut path = root.to_path_buf();
    let components: Vec<_> = Path::new(relative).components().collect();
    if components.is_empty() {
        return Err("empty selected source path".into());
    }
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err("selected source path escape refused".into());
        };
        path.push(name);
        let metadata = fs::symlink_metadata(&path)?;
        if !regular(&metadata)
            || (index + 1 < components.len() && !metadata.is_dir())
            || (index + 1 == components.len()
                && (!metadata.is_file() || metadata.len() > MAX_BYTES as u64))
        {
            return Err("selected source type or size refused".into());
        }
    }
    if path.canonicalize()? != path || !path.starts_with(root) {
        return Err("selected source containment refused".into());
    }
    // Metadata/open races are not prevented by this trusted-checkout reader.
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err("selected source read over budget".into());
    }
    Ok(bytes)
}

impl SourceBinding {
    pub(crate) fn bind(root: &Path, selected: &SelectedSource) -> Result<Self> {
        Self::with_profile(root, selected, profile())
    }

    fn with_profile(root: &Path, selected: &SelectedSource, profile: &'static Value) -> Result<Self> {
        let binding = Self {
            root: root.into(),
            selected: selected.clone(),
            profile,
            compose_joined: false,
        };
        binding.revalidate()?;
        Ok(binding)
    }

    fn validated_bytes(&self) -> Result<Vec<u8>> {
        let bytes = read(&self.root, &self.selected.path)?;
        let text = std::str::from_utf8(&bytes)?;
        if self.profile["schema"] != 1
            || self.profile["path"].as_str() != Some(self.selected.path.as_str())
            || self.profile["sha256"].as_str() != Some(self.selected.sha256.as_str())
            || self.profile["bytes"].as_u64() != Some(self.selected.bytes as u64)
            || bytes.len() != self.selected.bytes
            || super::digest(&bytes) != self.selected.sha256
            || self.profile["name"].as_str().is_none_or(str::is_empty)
        {
            return Err("selected source identity drift".into());
        }
        let mut lines = HashSet::new();
        let mut codes = HashSet::new();
        let sites = self.profile["sites"]
            .as_array()
            .ok_or("source sites absent")?;
        if sites.is_empty() || sites.len() > 128 {
            return Err("source sites count refused".into());
        }
        for site in sites {
            let line = site["line"].as_u64().ok_or("source line absent")?;
            let needle = site["needle"].as_str().ok_or("source needle absent")?;
            let code = site["code"].as_str().ok_or("source code absent")?;
            if line == 0
                || line > 999
                || needle.is_empty()
                || needle.len() > 128
                || !lines.insert(line)
                || !codes.insert(code)
                || !text
                    .lines()
                    .nth(line as usize - 1)
                    .is_some_and(|text| text.contains(needle))
            {
                return Err("selected source line profile drift".into());
            }
        }
        Ok(bytes)
    }

    pub(crate) fn revalidate(&self) -> Result<()> {
        self.validated_bytes().map(|_| ())
    }

    pub(crate) fn join_compose(&mut self, compose: &Value) -> Result<()> {
        self.compose_joined = false;
        for service in ["browser-gate", "browser-product"] {
            let context = compose["services"][service]["build"]["context"]
                .as_str()
                .ok_or("browser build context absent")?;
            if Path::new(context) != self.root.as_path()
                || Path::new(context).canonicalize()? != self.root
            {
                return Err("browser build context differs from selected source root".into());
            }
        }
        self.revalidate()?;
        self.compose_joined = true;
        Ok(())
    }

    pub(crate) fn refinement_ready(&self) -> bool {
        self.compose_joined && self.revalidate().is_ok()
    }

    pub(crate) fn path(&self) -> &'static str {
        let profile = self.profile;
        profile["path"].as_str().expect("validated source path")
    }

    pub(crate) fn name(&self) -> &'static str {
        let profile = self.profile;
        profile["name"].as_str().expect("validated selected name")
    }

    pub(crate) fn site(&self, line: usize, column: usize) -> Option<&'static str> {
        let bytes = self.validated_bytes().ok()?;
        let text = std::str::from_utf8(&bytes).ok()?;
        if column == 0 || column > text.lines().nth(line.checked_sub(1)?)?.len() {
            return None;
        }
        let profile = self.profile;
        profile["sites"].as_array()?.iter().find_map(|site| {
            if site["line"].as_u64() == Some(line as u64) {
                site["code"].as_str()
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
#[path = "source_pin/tests.rs"]
pub(crate) mod tests;
