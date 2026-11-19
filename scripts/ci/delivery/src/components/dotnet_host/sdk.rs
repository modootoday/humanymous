//! Pinned .NET SDK custody for the dotnet host provider. The provider
//! downloads the one pinned linux-x64 SDK tarball from Microsoft's official
//! build host into a private directory, checks its size and SHA-512 against
//! the pins below before extraction, and runs every SDK command with that
//! private root and with `adapters/dotnet` as the working directory, so
//! `adapters/dotnet/global.json` (10.0.403, `rollForward: disable`) applies.
//! `dotnet --version` must then print exactly the pinned version. Any
//! mismatch fails closed; a runner-installed SDK is never used.
use crate::{Result, require, source};
use serde_json::{Value, json};
use sha2::{Digest, Sha512};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

/// The SDK version `adapters/dotnet/global.json` pins.
pub const VERSION: &str = "10.0.403";
/// Microsoft's official release metadata that publishes both digests below:
/// `releases[].sdks[version == "10.0.403"].files[rid].hash` (release
/// 10.0.14, 2026-10-25). linux-x64 `.tar.gz` read 2026-11-18; win-x64 `.zip`
/// read 2026-11-19.
pub const METADATA_URL: &str =
    "https://builds.dotnet.microsoft.com/dotnet/release-metadata/10.0/releases.json";

/// One official SDK archive: URL, SHA-512 copied from [`METADATA_URL`], and
/// the exact observed length (a different length never verifies).
pub struct Pin {
    pub rid: &'static str,
    pub url: &'static str,
    pub sha512: &'static str,
    pub size: u64,
    /// Archive file name inside the private directory.
    pub archive: &'static str,
    /// Muxer path relative to the extracted root.
    pub muxer: &'static str,
}
pub const LINUX_X64: Pin = Pin {
    rid: "linux-x64",
    url: "https://builds.dotnet.microsoft.com/dotnet/Sdk/10.0.403/dotnet-sdk-10.0.403-linux-x64.tar.gz",
    sha512: "b7b3d8461cf98b07d2552ce9199702fa3ad010ac2671a5f8cd5136f7534e50f6855ba7bbb02b11494563f1d04156276a31cda1103b900e5eecd0d9473dc7b9c5",
    size: 240_199_984,
    archive: "dotnet-sdk.tar.gz",
    muxer: "dotnet",
};
pub const WIN_X64: Pin = Pin {
    rid: "win-x64",
    url: "https://builds.dotnet.microsoft.com/dotnet/Sdk/10.0.403/dotnet-sdk-10.0.403-win-x64.zip",
    sha512: "7a114c318fa7f1d9822991b534b2814d0e78386ceda816b371f5141acefe741d87ccb606ede06cadac8f9ac5070e32b708a67bbd1619ff5e5a7858a27874ef48",
    size: 300_745_255,
    archive: "dotnet-sdk.zip",
    muxer: "dotnet.exe",
};
/// The archive this host's provider acquires.
pub const PIN: Pin = if cfg!(windows) { WIN_X64 } else { LINUX_X64 };
/// Kept for the declared pilot and existing callers: this host's archive URL.
pub const URL: &str = PIN.url;
pub const SHA512: &str = PIN.sha512;
pub const SIZE: u64 = PIN.size;
const DOWNLOAD_BUDGET: Duration = Duration::from_secs(600);

/// A verified, extracted private SDK root.
pub struct Sdk {
    pub root: PathBuf,
    pub muxer: PathBuf,
}

/// Stream `reader` into `writer`, refusing any length other than `size` and
/// any SHA-512 other than `sha512`. The caller must discard the written bytes
/// on error. Returns the observed lowercase digest.
pub fn copy_verified(
    reader: impl Read,
    mut writer: impl Write,
    size: u64,
    sha512: &str,
) -> Result<String> {
    let mut hasher = Sha512::new();
    let mut reader = reader.take(size + 1);
    let mut buffer = vec![0_u8; 1 << 16];
    let mut total = 0_u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        total += read as u64;
        require(total <= size, ".NET SDK download longer than pinned size")?;
        hasher.update(&buffer[..read]);
        writer
            .write_all(&buffer[..read])
            .map_err(|e| e.to_string())?;
    }
    require(total == size, ".NET SDK download size differs from pin")?;
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    require(
        digest == sha512,
        ".NET SDK SHA-512 differs from the pinned Microsoft release metadata",
    )?;
    writer.flush().map_err(|e| e.to_string())?;
    Ok(digest)
}

#[cfg_attr(windows, allow(dead_code, reason = "Windows unpacks the pinned zip"))]
/// Extract a verified gzip tarball into the empty `destination`. Only plain
/// relative regular files and directories are accepted: the pinned SDK has no
/// links, devices or absolute members.
pub fn extract(tarball: impl Read, destination: &Path) -> Result<u64> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    archive.set_preserve_permissions(false);
    archive.set_unpack_xattrs(false);
    let mut files = 0_u64;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        require(
            kind.is_file() || kind.is_dir(),
            ".NET SDK tarball member is not a regular file or directory",
        )?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        require(
            path.components()
                .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
            ".NET SDK tarball member path is not plain relative",
        )?;
        require(
            entry.unpack_in(destination).map_err(|e| e.to_string())?,
            ".NET SDK tarball member escapes the destination",
        )?;
        files += u64::from(kind.is_file());
    }
    Ok(files)
}

/// `dotnet --version` output must be exactly the pinned version: one line,
/// nothing else. A list that merely contains it never qualifies.
pub fn exact_version(output: &str) -> Result<()> {
    let lines: Vec<_> = output.lines().filter(|line| !line.is_empty()).collect();
    require(
        lines == [VERSION],
        format!(
            "pinned .NET SDK {VERSION} required; `dotnet --version` resolved {:?}",
            output.trim()
        ),
    )
}

/// A new owner-only (0600) file; it must not already exist.
#[cfg(unix)]
fn owner_only_file(path: &Path) -> Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!(".NET SDK tarball: {e}"))
}
/// Windows: a new file in the administrator's private provider directory. The
/// runner re-verifies every staged runtime byte against its own pin.
#[cfg(windows)]
fn owner_only_file(path: &Path) -> Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!(".NET SDK archive: {e}"))
}
#[cfg(not(any(unix, windows)))]
fn owner_only_file(_path: &Path) -> Result<std::fs::File> {
    super::unsupported()
}
#[cfg(windows)]
fn sdk_root(root: &Path) -> Result<()> {
    std::fs::create_dir(root).map_err(|e| format!(".NET SDK root: {e}"))
}
#[cfg(not(windows))]
fn sdk_root(root: &Path) -> Result<()> {
    super::private_dir(root)
}
#[cfg(windows)]
fn unpack(archive: &Path, root: &Path) -> Result<u64> {
    super::zip::extract(archive, root)
}
#[cfg(not(windows))]
fn unpack(archive: &Path, root: &Path) -> Result<u64> {
    extract(
        std::fs::File::open(archive).map_err(|e| e.to_string())?,
        root,
    )
}

/// Download, verify and extract the pinned SDK into `private`, which must be
/// a fresh owner-only directory. Records the custody facts in `report`.
pub fn acquire(private: &Path, report: &mut Value) -> Result<Sdk> {
    let tarball = private.join(PIN.archive);
    let root = private.join("sdk");
    sdk_root(&root)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(DOWNLOAD_BUDGET))
        .build()
        .into();
    let response = agent
        .get(URL)
        .call()
        .map_err(|e| format!(".NET SDK download: {e}"))?;
    let body = response
        .into_body()
        .into_with_config()
        .limit(SIZE + 1)
        .reader();
    let file = owner_only_file(&tarball)?;
    let verified = copy_verified(body, std::io::BufWriter::new(file), SIZE, SHA512);
    let digest = match verified {
        Ok(digest) => digest,
        Err(error) => {
            let _ = std::fs::remove_file(&tarball);
            return Err(error);
        }
    };
    let files = unpack(&tarball, &root)?;
    std::fs::remove_file(&tarball).map_err(|e| e.to_string())?;
    let muxer = root.join(PIN.muxer);
    require(muxer.is_file(), ".NET SDK muxer absent after extraction")?;
    report["dotnet_sdk"] = json!({
        "version_pinned": VERSION,
        "rid": PIN.rid,
        "source_url": URL,
        "metadata_url": METADATA_URL,
        "tarball_size": SIZE,
        "tarball_sha512": digest,
        "tarball_sha512_verified": true,
        "extracted_files": files,
        "runner_sdk_used": false,
    });
    Ok(Sdk { root, muxer })
}

impl Sdk {
    /// Run one SDK command with the private root and `adapters/dotnet` as the
    /// working directory; DOTNET_ROOT and PATH are set only for this child.
    pub fn invoke(
        &self,
        root: &Path,
        dir: &Path,
        report: &mut Value,
        stage: &str,
        args: &[&str],
        budget: &str,
    ) -> Result<()> {
        report["stage"] = json!(stage);
        let muxer = super::text(&self.muxer)?;
        // Windows has no coreutils `timeout`; the job's own limit bounds it.
        let (program, fixed) = if cfg!(windows) {
            let _ = budget;
            (
                muxer.clone(),
                args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
            )
        } else {
            let mut fixed = vec!["--signal=TERM", "--kill-after=15s", budget, &muxer];
            fixed.extend_from_slice(args);
            (
                "timeout".to_owned(),
                crate::metrics::arguments("timeout", &fixed),
            )
        };
        let log = dir.join(format!("{stage}.log"));
        let output = std::fs::File::create(&log).map_err(|e| e.to_string())?;
        let error =
            std::fs::File::create(log.with_extension("stderr.log")).map_err(|e| e.to_string())?;
        let mut path = OsString::from(self.root.as_os_str());
        if let Some(inherited) = std::env::var_os("PATH") {
            path.push(if cfg!(windows) { ";" } else { ":" });
            path.push(inherited);
        }
        let measurement = crate::metrics::Measurement::start(root, &program, &fixed);
        let status = Command::new(&program)
            .args(&fixed)
            .current_dir(root.join("adapters/dotnet"))
            .env_remove("DOTNET_ROOT_X64")
            .env_remove("MSBuildSDKsPath")
            .env_remove("MSBuildExtensionsPath")
            .env_remove("DOTNET_HOST_PATH")
            .env("DOTNET_ROOT", &self.root)
            .env("PATH", path)
            .env("DOTNET_MULTILEVEL_LOOKUP", "0")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
            .stdin(Stdio::null())
            .stdout(output)
            .stderr(error)
            .status();
        measurement.finish(root, status.as_ref().ok().copied());
        let code = status.map_err(|e| e.to_string())?.code().unwrap_or(-1);
        report["commands"][stage] = json!({"exit_code":code,"stdout_sha256":source::file(&log)?,"stderr_sha256":source::file(&log.with_extension("stderr.log"))?});
        require(code == 0, "owned component command failed")
    }

    /// Resolve the SDK the pinned working directory selects and require it to
    /// be exactly [`VERSION`]; record what actually resolved.
    pub fn check(&self, root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
        self.invoke(root, dir, report, "dotnet-version", &["--version"], "60s")?;
        let resolved =
            std::fs::read_to_string(dir.join("dotnet-version.log")).map_err(|e| e.to_string())?;
        report["dotnet_sdk"]["version_resolved"] = json!(resolved.trim());
        report["dotnet_sdk"]["muxer_sha256"] = json!(source::file(&self.muxer)?);
        exact_version(&resolved)?;
        self.invoke(root, dir, report, "dotnet-info", &["--info"], "60s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sha512(bytes: &[u8]) -> String {
        Sha512::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
    #[test]
    fn pins_name_one_official_archive_and_digest_per_rid() {
        for (pin, suffix) in [(&LINUX_X64, "linux-x64.tar.gz"), (&WIN_X64, "win-x64.zip")] {
            assert!(
                pin.url
                    .starts_with("https://builds.dotnet.microsoft.com/dotnet/Sdk/10.0.403/")
            );
            assert!(pin.url.ends_with(&format!("dotnet-sdk-{VERSION}-{suffix}")));
            assert!(pin.url.contains(pin.rid));
            assert_eq!(pin.sha512.len(), 128);
            assert!(
                pin.sha512
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            );
        }
        assert_ne!(LINUX_X64.sha512, WIN_X64.sha512);
        assert_eq!(
            URL,
            if cfg!(windows) {
                WIN_X64.url
            } else {
                LINUX_X64.url
            }
        );
        assert!(METADATA_URL.starts_with("https://builds.dotnet.microsoft.com/"));
        let global: Value = serde_json::from_str(include_str!(
            "../../../../../../adapters/dotnet/global.json"
        ))
        .unwrap();
        assert_eq!(global["sdk"]["version"], VERSION);
        assert_eq!(global["sdk"]["rollForward"], "disable");
    }
    #[test]
    fn exact_version_accepts_only_the_pinned_sdk() {
        assert!(exact_version("10.0.403\n").is_ok());
        assert!(exact_version("10.0.403").is_ok());
        assert!(exact_version("11.0.100\n").is_err());
        assert!(exact_version("10.0.404\n").is_err());
        assert!(exact_version("10.0.4030\n").is_err());
        assert!(exact_version("").is_err());
        // An installed-SDK list that contains the pin but resolves elsewhere.
        let list = "10.0.403 [/usr/share/dotnet/sdk]\n11.0.100 [/usr/share/dotnet/sdk]\n";
        assert!(exact_version(list).is_err());
        let info = ".NET SDK:\n Version:           11.0.100\n\n.NET SDKs installed:\n  10.0.403 [/usr/share/dotnet/sdk]\n  11.0.100 [/usr/share/dotnet/sdk]\n";
        assert!(exact_version(info).is_err());
        assert!(exact_version("11.0.100\n10.0.403\n").is_err());
    }
    #[test]
    fn copy_verified_refuses_digest_and_size_mismatch() {
        let bytes = b"pinned sdk fixture".as_slice();
        let digest = sha512(bytes);
        let mut out = Vec::new();
        assert_eq!(
            copy_verified(bytes, &mut out, bytes.len() as u64, &digest).unwrap(),
            digest
        );
        assert_eq!(out, bytes);
        let wrong = sha512(b"other");
        assert!(copy_verified(bytes, Vec::new(), bytes.len() as u64, &wrong).is_err());
        assert!(copy_verified(bytes, Vec::new(), bytes.len() as u64 - 1, &digest).is_err());
        assert!(copy_verified(bytes, Vec::new(), bytes.len() as u64 + 1, &digest).is_err());
        let tampered = b"pinned sdk fixturE".as_slice();
        assert!(copy_verified(tampered, Vec::new(), tampered.len() as u64, &digest).is_err());
    }
    fn tarball(rows: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, kind, bytes) in rows {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(*kind);
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            if *kind == tar::EntryType::Symlink {
                header.set_link_name("/etc/passwd").unwrap();
            }
            // Bypass the builder's own path validation to model a hostile member.
            let name = &mut header.as_old_mut().name;
            name[..path.len()].copy_from_slice(path.as_bytes());
            header.set_cksum();
            builder.append(&header, *bytes).unwrap();
        }
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&builder.into_inner().unwrap()).unwrap();
        encoder.finish().unwrap()
    }
    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hm-sdk-test-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        dir
    }
    #[test]
    fn extract_accepts_plain_members_and_refuses_links_and_escapes() {
        let good = tarball(&[
            ("sdk/", tar::EntryType::Directory, b""),
            ("dotnet", tar::EntryType::Regular, b"muxer"),
            ("sdk/10.0.403/x.dll", tar::EntryType::Regular, b"x"),
        ]);
        let dir = scratch("good");
        assert_eq!(extract(good.as_slice(), &dir).unwrap(), 2);
        assert_eq!(std::fs::read(dir.join("dotnet")).unwrap(), b"muxer");
        std::fs::remove_dir_all(&dir).unwrap();
        for (label, row) in [
            ("link", ("dotnet", tar::EntryType::Symlink, b"".as_slice())),
            ("hard", ("dotnet", tar::EntryType::Link, b"".as_slice())),
            (
                "parent",
                ("../escape", tar::EntryType::Regular, b"x".as_slice()),
            ),
            (
                "absolute",
                ("/tmp/escape", tar::EntryType::Regular, b"x".as_slice()),
            ),
        ] {
            let dir = scratch(label);
            assert!(
                extract(tarball(&[row]).as_slice(), &dir).is_err(),
                "{label}"
            );
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }
    /// Actual download from Microsoft; run on demand where the host is reachable.
    #[test]
    #[ignore = "downloads the 240 MB pinned SDK from builds.dotnet.microsoft.com"]
    fn actual_pinned_sdk_download_verifies() {
        let dir = scratch("actual");
        let mut report = json!({});
        let sdk = acquire(&dir, &mut report).unwrap();
        assert!(sdk.muxer.is_file());
        assert_eq!(report["dotnet_sdk"]["tarball_sha512"], SHA512);
        // The pinned working directory resolves exactly the pinned SDK.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let logs = dir.join("logs");
        std::fs::create_dir(&logs).unwrap();
        sdk.check(&root, &logs, &mut report).unwrap();
        assert_eq!(report["dotnet_sdk"]["version_resolved"], VERSION);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
