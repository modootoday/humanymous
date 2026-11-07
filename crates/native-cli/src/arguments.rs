//! Bounded, explicit command-line selection with a read-only default.
use std::path::PathBuf;

/// Supported planning targets; every adapter is still unimplemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    /// Electron main-process Rust Node-API addon.
    Electron,
    /// Tauri Rust backend plugin.
    Tauri,
    /// Owned-window Rust event-loop adapter.
    Winit,
    /// External .NET C ABI consumer.
    Dotnet,
    /// External Qt C ABI consumer.
    Qt,
    /// External Flutter C ABI consumer.
    Flutter,
}

/// Strictly parsed init selection. No I/O or environment inspection in the parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Init {
    /// Selected integration family.
    pub framework: Framework,
    /// Requested Rust target triple; missing means unspecified, never auto-qualified.
    pub target: Option<String>,
    /// App-supplied runtime version; missing remains unspecified.
    pub runtime_version: Option<String>,
    /// Explicit root required only for write. Dry-run does not inspect it.
    pub workspace: Option<PathBuf>,
    /// Direct-child JSON filename.
    pub output: String,
    /// Explicit permission to create one new manifest; default false.
    pub write: bool,
}

/// Available command variants; future doctor/verify commands are not implemented.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Describe implemented commands and limits.
    Help,
    /// Print this tool's package version.
    Version,
    /// Plan or create a native integration manifest.
    Init(Init),
}

fn value(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let value = args.next().ok_or("missing option value")?;
    if value.is_empty() || value.starts_with('-') {
        return Err("missing value: option tokens cannot be values".into());
    }
    Ok(value)
}

/// Parse at most32 bounded UTF-8 arguments, excluding the executable name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut bounded = Vec::new();
    for argument in args {
        if bounded.len() == 32 || argument.len() > 4096 || argument.contains('\0') {
            return Err("argument budget exceeded".into());
        }
        bounded.push(argument);
    }
    if bounded == ["--help"] || bounded == ["-h"] {
        return Ok(Command::Help);
    }
    if bounded == ["--version"] {
        return Ok(Command::Version);
    }
    let mut args = bounded.into_iter();
    if args.next().as_deref() != Some("init") {
        return Err("expected init; see --help".into());
    }
    let (mut framework, mut target, mut runtime_version, mut workspace) = (None, None, None, None);
    let (mut output, mut mode) = (None, None);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--framework" if framework.is_none() => {
                framework = Some(match value(&mut args)?.as_str() {
                    "electron" => Framework::Electron,
                    "tauri" => Framework::Tauri,
                    "winit" => Framework::Winit,
                    "dotnet" => Framework::Dotnet,
                    "qt" => Framework::Qt,
                    "flutter" => Framework::Flutter,
                    _ => return Err("unknown or missing framework".into()),
                });
            }
            "--target" if target.is_none() => {
                let value = value(&mut args)?;
                if ![
                    "x86_64-pc-windows-msvc",
                    "aarch64-pc-windows-msvc",
                    "x86_64-unknown-linux-gnu",
                    "aarch64-unknown-linux-gnu",
                    "x86_64-apple-darwin",
                    "aarch64-apple-darwin",
                ]
                .contains(&value.as_str())
                {
                    return Err("unknown planning target".into());
                }
                target = Some(value);
            }
            "--runtime-version" if runtime_version.is_none() => {
                let value = value(&mut args)?;
                if !value
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                    || value.len() > 64
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
                {
                    return Err("runtime version must be a bounded version token".into());
                }
                runtime_version = Some(value);
            }
            "--workspace" if workspace.is_none() => {
                workspace = Some(PathBuf::from(value(&mut args)?))
            }
            "--output" if output.is_none() => output = Some(value(&mut args)?),
            "--write" if mode.is_none() => mode = Some(true),
            "--dry-run" if mode.is_none() => mode = Some(false),
            _ => return Err("unknown, duplicate or conflicting option".into()),
        }
    }
    let output = output.unwrap_or_else(|| "humanymous-native.json".into());
    validate_output(&output)?;
    let write = mode.unwrap_or(false);
    if write && workspace.is_none() {
        return Err("--write requires explicit --workspace".into());
    }
    Ok(Command::Init(Init {
        framework: framework.ok_or("--framework required")?,
        target,
        runtime_version,
        workspace,
        output,
        write,
    }))
}

/// Validate one portable JSON basename, including Windows reserved-device rules.
pub fn validate_output(name: &str) -> Result<(), String> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if name.len() > 80
        || !name.ends_with(".json")
        || stem.is_empty()
        || reserved.contains(&stem.as_str())
        || !name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err("output must be a nonreserved direct-child ASCII .json basename".into());
    }
    Ok(())
}
