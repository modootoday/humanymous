//! Fresh reconstruction beneath a verified private parent. Trusted ancestors and
//! same-owner nonreplacement remain premises; no complete subtree lease is claimed.
use crate::{Result, require, source};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

fn directory(path: &Path) -> Result<File> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "private proof directory missing")?;
    require(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "private proof directory type refused",
    )?;
    #[cfg(windows)]
    let file = {
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(0x02200000)
            .open(path)
    };
    #[cfg(not(windows))]
    let file = File::open(path);
    let file = file.map_err(|_| "private proof directory open failed")?;
    verify(&file, true)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let held = file
            .metadata()
            .map_err(|_| "private proof held directory metadata absent")?;
        require(
            held.dev() == metadata.dev() && held.ino() == metadata.ino(),
            "private proof directory changed while opening",
        )?;
    }
    Ok(file)
}
fn verify(file: &File, is_directory: bool) -> Result<()> {
    #[cfg(windows)]
    {
        use humanymous_native_security::{
            process,
            storage::{self, WindowsPrivacy},
        };
        let identity =
            process::current_identity().map_err(|_| "private proof actor unavailable")?;
        require(
            identity.installer_allowed(),
            "private proof ordinary actor required",
        )?;
        let policy = if is_directory {
            WindowsPrivacy::InheritableDirectory
        } else {
            WindowsPrivacy::File
        };
        storage::verify_windows_privacy(file, identity.sid(), policy)
            .map_err(|_| "private proof held object privacy refused")?;
        if is_directory {
            storage::verify_windows_privacy(
                file,
                identity.sid(),
                WindowsPrivacy::ProtectedDirectory,
            )
            .map_err(|_| "private proof held directory protection refused")?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let metadata = file
            .metadata()
            .map_err(|_| "private proof metadata absent")?;
        require(
            metadata.uid() == humanymous_native_security::process::current_uid()
                && metadata.permissions().mode() & 0o7777
                    == if is_directory { 0o700 } else { 0o600 }
                && if is_directory {
                    metadata.is_dir()
                } else {
                    metadata.is_file() && metadata.nlink() == 1
                },
            "private proof held owner or mode refused",
        )?;
    }
    #[cfg(not(any(unix, windows)))]
    return Err("private proof provider unavailable".into());
    Ok(())
}
fn child(parent: &Path, name: &str) -> Result<PathBuf> {
    require(
        !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "private proof child name refused",
    )?;
    #[cfg(windows)]
    let path = humanymous_native_security::storage::create_user_private_directory(parent, name)
        .map_err(|_| "private proof fresh child creation refused")?;
    #[cfg(unix)]
    let path = {
        use std::os::unix::fs::DirBuilderExt;
        let path = parent.join(name);
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| "private proof fresh child creation refused")?;
        path
    };
    #[cfg(not(any(unix, windows)))]
    return Err("private proof provider unavailable".into());
    Ok(path)
}
fn write(path: &Path, bytes: &[u8]) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x00200000);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "private proof fresh file creation refused")?;
    verify(&file, false)?;
    file.write_all(bytes)
        .map_err(|_| "private proof original byte write failed")?;
    file.sync_all()
        .map_err(|_| "private proof file sync failed")?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "private proof readback seek failed")?;
    let mut readback = Vec::new();
    (&mut file)
        .take(bytes.len() as u64 + 1)
        .read_to_end(&mut readback)
        .map_err(|_| "private proof readback failed")?;
    require(
        readback == bytes,
        "private proof original byte readback mismatch",
    )?;
    verify(&file, false)?;
    Ok(file)
}

pub(super) fn reconstruct(
    parent: &Path,
    name: &str,
    paths: &[String],
    files: &[Vec<u8>],
    receipt: &serde_json::Value,
) -> Result<PathBuf> {
    require(
        parent.is_absolute() && paths.len() == 16 && files.len() == 16,
        "private proof reconstruction arguments refused",
    )?;
    let _parent = directory(parent)?;
    let root = child(parent, name)?;
    let root_handle = directory(&root)?;
    let compiler = child(&root, "header-compiler")?;
    let compiler_handle = directory(&compiler)?;
    let mut held = Vec::new();
    for (path, bytes) in paths.iter().zip(files) {
        require(
            source::safe_relative(path)
                && (path.starts_with("header-compiler/") || !path.contains('/')),
            "private proof fixed file path refused",
        )?;
        held.push(write(&root.join(path), bytes)?);
    }
    for file in &held {
        verify(file, false)?;
    }
    verify(&root_handle, true)?;
    verify(&compiler_handle, true)?;
    #[cfg(unix)]
    {
        compiler_handle
            .sync_all()
            .map_err(|_| "private proof compiler directory sync failed")?;
        root_handle
            .sync_all()
            .map_err(|_| "private proof root directory sync failed")?;
    }
    let marker = serde_json::to_vec_pretty(receipt)
        .map_err(|_| "private proof completion encoding failed")?;
    let pending = root.join("proof-complete.pending.json");
    let completion = write(&pending, &marker)?;
    drop(completion);
    // Promotion is the final fallible operation. The private fresh-root premise
    // excludes another same-owner writer; no crash durability after rename is claimed.
    fs::rename(&pending, root.join("proof-complete.json"))
        .map_err(|_| "private proof completion promotion failed")?;
    Ok(root)
}
