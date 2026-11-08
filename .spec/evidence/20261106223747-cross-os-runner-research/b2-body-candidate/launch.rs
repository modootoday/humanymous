//! Closed qualification-helper DATA and actual held x64/CUI image inspection.
#![forbid(unsafe_code)]

use super::state::Issue;
use std::{ffi::OsStr, fs::{File, OpenOptions}, io::{Read, Seek, SeekFrom}, os::windows::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt}}, path::Path};

/// Fixed Rust fixture modes; no arbitrary program arguments or command callback.
/// The caller must independently admit the actual test-helper artifact/source.
#[derive(Clone, Copy)]
pub enum QualificationHelper {
    /// Fixed helper returns naturally.
    ExitZero,
    /// Fixed helper has actual terminal exit259.
    Exit259,
    /// Fixed helper waits until its OWN enclosing Job is stopped.
    Wait,
    /// Fixed helper leaves one ordinary CreateProcess descendant running.
    LeaveDescendant,
    /// Fixed helper creates a nested owned tree then exits without Drop.
    OwnerDies,
    /// Fixed helper probes detached standard streams without native console APIs.
    ProbeStandardStreams,
}
impl QualificationHelper {
    fn selector(self) -> &'static str {
        match self {
            Self::ExitZero => "windows_supervisor::native_tests::child_exit_zero",
            Self::Exit259 => "windows_supervisor::native_tests::child_exit_259",
            Self::Wait => "windows_supervisor::native_tests::child_wait",
            Self::LeaveDescendant => "windows_supervisor::native_tests::child_leaves_descendant",
            Self::OwnerDies => "windows_supervisor::native_tests::child_owner_dies",
            Self::ProbeStandardStreams => "windows_supervisor::native_tests::child_probes_standard_streams",
        }
    }
}

pub(super) struct PreparedLaunch {
    pub(super) image: Vec<u16>,
    pub(super) command: Vec<u16>,
    pub(super) cwd: Vec<u16>,
    pub(super) environment: [u16; 2],
    _image_guard: File,
    _cwd_guard: File,
}

fn wide(value: &OsStr) -> Result<Vec<u16>, Issue> {
    let mut units: Vec<_> = value.encode_wide().take(32768).collect();
    if units.is_empty() || units.len() >= 32768 || units.contains(&0) { return Err(Issue::InvalidInput); }
    units.push(0);
    Ok(units)
}

fn local_locator(path: &Path) -> Result<(), Issue> {
    let units: Vec<_> = path.as_os_str().encode_wide().take(32768).collect();
    if units.len() < 4 || units.len() >= 32768
        || !((65..=90).contains(&units[0]) || (97..=122).contains(&units[0]))
        || units[1..3] != [58, 92]
        || units[3..].split(|unit| *unit == 92).any(|part| part.is_empty()
            || part == [46] || part == [46, 46]
            || part.last().is_some_and(|unit| *unit == 32 || *unit == 46)
            || part.iter().any(|unit| *unit < 32 || [34, 42, 47, 58, 60, 62, 63, 124].contains(unit)))
    { return Err(Issue::InvalidInput); }
    Ok(())
}

fn quote(units: &[u16], output: &mut Vec<u16>) -> Result<(), Issue> {
    if units.contains(&0) || units.len() > 4096 { return Err(Issue::InvalidInput); }
    output.push(34);
    let mut slashes = 0;
    for unit in units {
        if *unit == 92 { slashes += 1; continue; }
        let count = if *unit == 34 { slashes * 2 + 1 } else { slashes };
        output.extend(std::iter::repeat_n(92, count));
        output.push(*unit);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(92, slashes * 2));
    output.push(34);
    if output.len() >= 32767 { return Err(Issue::InvalidInput); }
    Ok(())
}

fn cui_header(dos: &[u8; 64], coff: &[u8; 24], optional: &[u8; 70]) -> Result<(), Issue> {
    let machine = u16::from_le_bytes([coff[4], coff[5]]);
    let attributes = u16::from_le_bytes([coff[22], coff[23]]);
    if &dos[..2] != b"MZ" || &coff[..4] != b"PE\0\0" || machine != 0x8664
        || attributes & 0x2000 != 0 || u16::from_le_bytes([optional[0], optional[1]]) != 0x20b
        || u16::from_le_bytes([optional[68], optional[69]]) != 3
    { return Err(Issue::ImageProfile); }
    Ok(())
}

impl PreparedLaunch {
    pub(super) fn prepare(executable: &Path, cwd: &Path, helper: QualificationHelper) -> Result<Self, Issue> {
        if !executable.is_absolute() || !cwd.is_absolute() { return Err(Issue::InvalidInput); }
        local_locator(executable)?;
        local_locator(cwd)?;
        let mut image_guard = OpenOptions::new().access_mode(0x120089).share_mode(1)
            .custom_flags(0x00200000).open(executable).map_err(|_| Issue::ImageProfile)?;
        let metadata = image_guard.metadata().map_err(|_| Issue::ImageProfile)?;
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 || metadata.len() > 1024 * 1024 * 1024 {
            return Err(Issue::ImageProfile);
        }
        let mut dos = [0; 64];
        image_guard.read_exact(&mut dos).map_err(|_| Issue::ImageProfile)?;
        let offset = u32::from_le_bytes(dos[60..64].try_into().map_err(|_| Issue::ImageProfile)?) as u64;
        if !(64..=1024 * 1024).contains(&offset) { return Err(Issue::ImageProfile); }
        image_guard.seek(SeekFrom::Start(offset)).map_err(|_| Issue::ImageProfile)?;
        let mut coff = [0; 24];
        image_guard.read_exact(&mut coff).map_err(|_| Issue::ImageProfile)?;
        let optional_size = u16::from_le_bytes([coff[20], coff[21]]) as u64;
        if !(70..=4096).contains(&optional_size) || offset + 24 + optional_size > metadata.len() { return Err(Issue::ImageProfile); }
        let mut optional = [0; 70];
        image_guard.read_exact(&mut optional).map_err(|_| Issue::ImageProfile)?;
        cui_header(&dos, &coff, &optional)?;
        let cwd_guard = OpenOptions::new().access_mode(0x20001).share_mode(3)
            .custom_flags(0x02200000).open(cwd).map_err(|_| Issue::InvalidInput)?;
        let metadata = cwd_guard.metadata().map_err(|_| Issue::InvalidInput)?;
        if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 { return Err(Issue::InvalidInput); }
        let image = wide(executable.as_os_str())?;
        let cwd = wide(cwd.as_os_str())?;
        let mut command = Vec::new();
        quote(&image[..image.len() - 1], &mut command)?;
        for argument in ["--exact", helper.selector(), "--ignored", "--test-threads=1"] {
            command.push(32);
            quote(&argument.encode_utf16().collect::<Vec<_>>(), &mut command)?;
        }
        command.push(0);
        Ok(Self { image, command, cwd, environment: [0, 0], _image_guard: image_guard, _cwd_guard: cwd_guard })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_quotes_unicode_backslashes_and_quotes_without_extra_arguments() {
        let mut output = Vec::new();
        quote(&"資料\\\"tail\\".encode_utf16().collect::<Vec<_>>(), &mut output).unwrap();
        let mut expected = vec![34]; expected.extend("資料".encode_utf16());
        expected.extend([92, 92, 92, 34]); expected.extend("tail".encode_utf16()); expected.extend([92, 92, 34]);
        assert_eq!(output, expected);
        assert!(quote(&[0], &mut Vec::new()).is_err());
        assert!(wide(OsStr::new("a\0b")).is_err());
        assert!(local_locator(Path::new(r"C:\資料\fixture.exe")).is_ok());
        for path in [r"\\server\share\fixture.exe", r"C:\a\..\fixture.exe", r"C:\a\x:stream", r"C:\a\tail."] {
            assert!(local_locator(Path::new(path)).is_err());
        }
    }
    #[test]
    fn held_image_profile_rejects_gui_dll_machine_and_optional_magic() {
        let mut dos = [0; 64]; dos[..2].copy_from_slice(b"MZ");
        let mut coff = [0; 24]; coff[..4].copy_from_slice(b"PE\0\0"); coff[4..6].copy_from_slice(&0x8664_u16.to_le_bytes());
        let mut optional = [0; 70]; optional[..2].copy_from_slice(&0x20b_u16.to_le_bytes()); optional[68] = 3;
        cui_header(&dos, &coff, &optional).unwrap();
        optional[68] = 2; assert!(cui_header(&dos, &coff, &optional).is_err()); optional[68] = 3;
        coff[23] = 0x20; assert!(cui_header(&dos, &coff, &optional).is_err()); coff[23] = 0;
        coff[4] = 0; assert!(cui_header(&dos, &coff, &optional).is_err());
        coff[4..6].copy_from_slice(&0x8664_u16.to_le_bytes()); optional[0] = 0;
        assert!(cui_header(&dos, &coff, &optional).is_err());
    }
}
