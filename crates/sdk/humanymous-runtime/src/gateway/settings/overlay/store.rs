//! Active overlay plus last-known-good file persistence and the static schema,
//! equal to the oracle `FileStore` and `Schema`.
//!
//! Boot recovery: a corrupt or invalid active file falls back to a valid LKG,
//! otherwise to no overlay; a missing active file never consults the LKG.
use super::wire::Json;
use super::{
    INTEGRITY_CRITICAL_ENGINE_HRS, INTEGRITY_CRITICAL_GATES, KNOWN_GATES, NET_POLICY_CLASSES,
    Overlay, OverlayDecodeError, OverlayEncodeError, OverlayValidationError, scoring_defaults,
    validate,
};
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

const ACTIVE_FILE: &str = "settings.overlay.v1.json";
const LKG_FILE: &str = "settings.overlay.lkg.json";

/// Why boot did not simply load a valid active overlay.
#[derive(Debug)]
pub enum OverlayLoadError {
    /// The active file could not be read.
    Read(std::io::Error),
    /// The active file is not a decodable overlay.
    Decode(OverlayDecodeError),
    /// The active file was corrupt (`Some`) or invalid (`None`) and the LKG was loaded.
    RecoveredFromLkg(Option<Box<OverlayLoadError>>),
    /// The active overlay failed validation and no valid LKG exists.
    InvalidWithoutLkg,
}

impl std::fmt::Display for OverlayLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "read active overlay: {error}"),
            Self::Decode(error) => error.fmt(formatter),
            Self::RecoveredFromLkg(Some(cause)) => {
                write!(formatter, "active corrupt; loaded LKG: {cause}")
            }
            // The oracle wraps a nil cause with %w, which formats this way.
            Self::RecoveredFromLkg(None) => {
                formatter.write_str("active corrupt; loaded LKG: %!w(<nil>)")
            }
            Self::InvalidWithoutLkg => formatter.write_str("active overlay invalid and no LKG"),
        }
    }
}

impl std::error::Error for OverlayLoadError {}

/// Store construction or activation failure.
#[derive(Debug)]
pub enum OverlayStoreError {
    /// The store directory path is empty.
    EmptyDirectory,
    /// The overlay failed validation; nothing was written.
    Invalid(OverlayValidationError),
    /// The overlay cannot be encoded; nothing was written.
    Encode(OverlayEncodeError),
    /// Directory creation or the active-file write failed.
    Io(std::io::Error),
}

impl std::fmt::Display for OverlayStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDirectory => formatter.write_str("settings: empty store dir"),
            Self::Invalid(error) => error.fmt(formatter),
            Self::Encode(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for OverlayStoreError {}

struct State {
    active: Option<Overlay>,
    /// Retained like the oracle's private last-known-good copy; not exposed.
    #[allow(dead_code)]
    last_known_good: Option<Overlay>,
    load_error: Option<OverlayLoadError>,
}

/// File-backed active overlay with last-known-good recovery.
pub struct OverlayFileStore {
    directory: PathBuf,
    state: RwLock<State>,
}

fn read_overlay(path: &Path) -> Result<Option<Overlay>, OverlayLoadError> {
    match std::fs::read(path) {
        Ok(bytes) => Overlay::from_json(&bytes)
            .map(Some)
            .map_err(OverlayLoadError::Decode),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(OverlayLoadError::Read(error)),
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)?.write_all(bytes)
}

impl OverlayFileStore {
    /// Open or create `directory` (mode 0700 on Unix) and recover the active
    /// overlay. Only an empty path or a directory-creation failure is an error;
    /// recovery problems are reported by [`Self::load_error`].
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, OverlayStoreError> {
        let directory = directory.as_ref();
        if directory.as_os_str().is_empty() {
            return Err(OverlayStoreError::EmptyDirectory);
        }
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(directory).map_err(OverlayStoreError::Io)?;
        let mut state = State {
            active: None,
            last_known_good: None,
            load_error: None,
        };
        let loaded = read_overlay(&directory.join(ACTIVE_FILE));
        let invalid = matches!(&loaded, Ok(Some(overlay)) if validate(Some(overlay)).is_err());
        match loaded {
            Ok(active) if !invalid => {
                state.last_known_good = active.clone();
                state.active = active;
            }
            loaded => {
                let cause = loaded.err();
                match read_overlay(&directory.join(LKG_FILE)) {
                    Ok(Some(lkg)) if validate(Some(&lkg)).is_ok() => {
                        state.active = Some(lkg.clone());
                        state.last_known_good = Some(lkg);
                        state.load_error =
                            Some(OverlayLoadError::RecoveredFromLkg(cause.map(Box::new)));
                    }
                    _ => {
                        state.load_error =
                            Some(cause.unwrap_or(OverlayLoadError::InvalidWithoutLkg));
                    }
                }
            }
        }
        Ok(Self {
            directory: directory.to_owned(),
            state: RwLock::new(state),
        })
    }

    /// In-memory active overlay; `None` is the empty overlay.
    pub fn active(&self) -> Option<Overlay> {
        self.state
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .active
            .clone()
    }

    /// Recovery diagnostic from boot, cleared by a successful activation.
    pub fn load_error(&self) -> Option<String> {
        self.state
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .load_error
            .as_ref()
            .map(ToString::to_string)
    }

    /// True when boot read or decoded the active file unsuccessfully, including
    /// a recovery from the LKG after such a failure.
    pub fn load_error_kind(&self) -> Option<&'static str> {
        let state = self.state.read().unwrap_or_else(PoisonError::into_inner);
        let kind = |error: &OverlayLoadError| match error {
            OverlayLoadError::Read(_) => "io",
            OverlayLoadError::Decode(_) => "decode",
            OverlayLoadError::RecoveredFromLkg(_) | OverlayLoadError::InvalidWithoutLkg => "other",
        };
        state.load_error.as_ref().map(|error| match error {
            OverlayLoadError::RecoveredFromLkg(Some(cause)) => kind(cause),
            other => kind(other),
        })
    }

    /// Validate and persist `overlay` as active and LKG; `None` removes only
    /// the active file. On success the caller's overlay status becomes
    /// `active`, as the oracle mutates it before encoding.
    pub fn set_active(&self, overlay: Option<&mut Overlay>) -> Result<(), OverlayStoreError> {
        let Some(overlay) = overlay else {
            let mut state = self.state.write().unwrap_or_else(PoisonError::into_inner);
            let _ = std::fs::remove_file(self.directory.join(ACTIVE_FILE));
            state.active = None;
            return Ok(());
        };
        validate(Some(overlay)).map_err(OverlayStoreError::Invalid)?;
        overlay.status = "active".into();
        let mut state = self.state.write().unwrap_or_else(PoisonError::into_inner);
        let bytes = overlay
            .to_json_indent()
            .map_err(OverlayStoreError::Encode)?;
        write_private(&self.directory.join(ACTIVE_FILE), &bytes).map_err(OverlayStoreError::Io)?;
        let _ = write_private(&self.directory.join(LKG_FILE), &bytes);
        state.active = Some(overlay.clone());
        state.last_known_good = Some(overlay.clone());
        state.load_error = None;
        Ok(())
    }
}

/// Go `json.Marshal` of the oracle static schema. The integrity-critical hard
/// rule list comes from a Go map there, so its order varies per process; this
/// port emits it sorted.
pub fn schema_json() -> Vec<u8> {
    let (challenge, deny, cap) = scoring_defaults();
    let bounds = |min: f64, max: f64, default: f64| {
        Json::Object(vec![
            ("default".into(), Json::Float(default)),
            ("max".into(), Json::Float(max)),
            ("min".into(), Json::Float(min)),
        ])
    };
    Json::Object(vec![
        (
            "emptyOverlayMeans".into(),
            Json::Str("no active overlay; preserve built-in and startup behavior".into()),
        ),
        ("gates".into(), Json::strings(KNOWN_GATES)),
        (
            "integrityCriticalGates".into(),
            Json::strings(INTEGRITY_CRITICAL_GATES),
        ),
        (
            "integrityCriticalHardRules".into(),
            Json::strings(INTEGRITY_CRITICAL_ENGINE_HRS),
        ),
        (
            "modes".into(),
            Json::Object(vec![
                (
                    "gates".into(),
                    Json::strings(["enforce", "monitor", "shadow", "off"]),
                ),
                ("hardRules".into(), Json::strings(["enforce", "monitor"])),
                (
                    "netPolicy".into(),
                    Json::strings(["enforce", "monitor", "shadow"]),
                ),
            ]),
        ),
        ("netPolicyClasses".into(), Json::strings(NET_POLICY_CLASSES)),
        (
            "notes".into(),
            Json::strings([
                "Detection measurements remain observable; enforcement changes are governed",
                "Transport-pattern evidence never auto-enforces",
                "Protection-reducing changes require a distinct Approver",
                "Integrity-reducing changes also require an explicit confirmation phrase",
                "Preview uses aggregates only and makes no numeric claim below 50 samples",
                "No active overlay preserves built-in and startup behavior",
                "Settings do not overcome the coherent real-browser and human-paced automation boundary",
            ]),
        ),
        ("schemaVersion".into(), Json::Str("1.0.0".into())),
        (
            "scoringBounds".into(),
            Json::Object(vec![
                ("challengeAt".into(), bounds(10.0, 60.0, challenge)),
                ("denyAt".into(), bounds(40.0, 95.0, deny)),
                ("layerCap".into(), bounds(30.0, 80.0, cap)),
            ]),
        ),
        (
            "weightMultiplier".into(),
            Json::Object(vec![
                ("max".into(), Json::Float(2.0)),
                ("min".into(), Json::Float(0.0)),
                ("step".into(), Json::Float(0.05)),
            ]),
        ),
    ])
    .encode(false)
    .expect("finite static schema")
    .into_bytes()
}
