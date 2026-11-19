//! Bounded extraction of the pinned win-x64 SDK zip after its SHA-512 is
//! verified. Only the archive shape the pinned file actually has is accepted:
//! no zip64, no encryption, stored or deflate members, plain relative names,
//! no links. Every member's CRC-32 and size must match its central record.
use crate::{Result, require};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path},
};

const EOCD: u32 = 0x0605_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;
/// More entries than any pinned SDK zip has; a larger directory is refused.
const MAX_ENTRIES: usize = 16_384;
/// Largest single member the pinned SDK has is far below this.
const MAX_MEMBER: u64 = 256 * 1024 * 1024;

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// One central-directory member.
struct Member {
    name: String,
    method: u16,
    crc: u32,
    compressed: u64,
    size: u64,
    offset: u64,
}

/// A member name must be plain relative forward-slash text.
pub fn plain_name(name: &str) -> bool {
    let trimmed = name.strip_suffix('/').unwrap_or(name);
    !trimmed.is_empty()
        && name.len() <= 512
        && !name.contains(['\\', ':', '\0'])
        && !name.chars().any(char::is_control)
        && Path::new(trimmed)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn central(file: &mut fs::File) -> Result<Vec<Member>> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    // No archive comment in the pinned zip: the 22-byte record ends the file.
    require(length >= 22, "SDK zip too short")?;
    let mut tail = [0_u8; 22];
    file.seek(SeekFrom::Start(length - 22))
        .and_then(|_| file.read_exact(&mut tail))
        .map_err(|e| e.to_string())?;
    require(
        u32_at(&tail, 0) == EOCD && u16_at(&tail, 20) == 0,
        "SDK zip end record absent or commented",
    )?;
    let (disk, start_disk) = (u16_at(&tail, 4), u16_at(&tail, 6));
    let (here, total) = (u16_at(&tail, 8), u16_at(&tail, 10));
    let (size, offset) = (u64::from(u32_at(&tail, 12)), u64::from(u32_at(&tail, 16)));
    require(
        disk == 0 && start_disk == 0 && here == total && total != u16::MAX,
        "SDK zip is multi-disk or zip64",
    )?;
    require(
        offset.checked_add(size) == Some(length - 22) && (total as usize) <= MAX_ENTRIES,
        "SDK zip central directory out of bounds",
    )?;
    let mut directory = vec![0_u8; size as usize];
    file.seek(SeekFrom::Start(offset))
        .and_then(|_| file.read_exact(&mut directory))
        .map_err(|e| e.to_string())?;
    let mut members = Vec::with_capacity(total as usize);
    let mut at = 0_usize;
    for _ in 0..total {
        require(
            at + 46 <= directory.len() && u32_at(&directory, at) == CENTRAL,
            "SDK zip central record malformed",
        )?;
        let flags = u16_at(&directory, at + 8);
        let method = u16_at(&directory, at + 10);
        let crc = u32_at(&directory, at + 16);
        let compressed = u64::from(u32_at(&directory, at + 20));
        let size = u64::from(u32_at(&directory, at + 24));
        let name_len = usize::from(u16_at(&directory, at + 28));
        let extra_len = usize::from(u16_at(&directory, at + 30));
        let comment_len = usize::from(u16_at(&directory, at + 32));
        let attributes = u32_at(&directory, at + 38);
        let local = u64::from(u32_at(&directory, at + 42));
        let end = at + 46 + name_len + extra_len + comment_len;
        require(end <= directory.len(), "SDK zip central record overruns")?;
        let name = std::str::from_utf8(&directory[at + 46..at + 46 + name_len])
            .map_err(|_| "SDK zip member name is not UTF-8")?
            .to_owned();
        // Bit 0 encryption, bit 6 strong encryption; 0xffffffff means zip64.
        require(
            flags & 0x41 == 0
                && matches!(method, 0 | 8)
                && compressed != u64::from(u32::MAX)
                && size != u64::from(u32::MAX)
                && local != u64::from(u32::MAX)
                && size <= MAX_MEMBER
                && (method != 0 || compressed == size),
            "SDK zip member is encrypted, zip64 or uses another method",
        )?;
        // Unix link mode in the high attribute half is never accepted.
        require(
            (attributes >> 16) & 0o170000 != 0o120000,
            "SDK zip member is a link",
        )?;
        require(
            plain_name(&name),
            "SDK zip member name is not plain relative",
        )?;
        members.push(Member {
            name,
            method,
            crc,
            compressed,
            size,
            offset: local,
        });
        at = end;
    }
    require(
        at == directory.len(),
        "SDK zip central directory has trailing bytes",
    )?;
    Ok(members)
}

/// Extract every member of a verified zip into the empty `destination`.
/// Returns the number of regular files written.
pub fn extract(path: &Path, destination: &Path) -> Result<u64> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    require(
        fs::read_dir(destination)
            .map_err(|e| e.to_string())?
            .next()
            .is_none(),
        "SDK extraction destination is not empty",
    )?;
    let members = central(&mut file)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut files = 0_u64;
    for member in members {
        require(
            seen.insert(member.name.to_ascii_lowercase()),
            "SDK zip has duplicate or case-colliding members",
        )?;
        let target = destination.join(member.name.trim_end_matches('/'));
        if member.name.ends_with('/') {
            require(member.size == 0, "SDK zip directory member has data")?;
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        let mut header = [0_u8; 30];
        file.seek(SeekFrom::Start(member.offset))
            .and_then(|_| file.read_exact(&mut header))
            .map_err(|e| e.to_string())?;
        require(
            u32_at(&header, 0) == LOCAL && u16_at(&header, 8) == member.method,
            "SDK zip local header disagrees with the central record",
        )?;
        let skip = u64::from(u16_at(&header, 26)) + u64::from(u16_at(&header, 28));
        file.seek(SeekFrom::Current(skip as i64))
            .map_err(|e| e.to_string())?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| format!("SDK member: {e}"))?;
        let raw = (&mut file).take(member.compressed);
        let mut reader: Box<dyn Read> = match member.method {
            0 => Box::new(raw),
            _ => Box::new(flate2::read::DeflateDecoder::new(raw)),
        };
        let mut crc = flate2::Crc::new();
        let mut buffer = vec![0_u8; 1 << 16];
        let mut written = 0_u64;
        loop {
            let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            written += read as u64;
            require(
                written <= member.size,
                "SDK zip member longer than recorded",
            )?;
            crc.update(&buffer[..read]);
            output
                .write_all(&buffer[..read])
                .map_err(|e| e.to_string())?;
        }
        require(
            written == member.size && crc.sum() == member.crc,
            "SDK zip member size or CRC-32 differs from its record",
        )?;
        files += 1;
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Minimal stored/deflate zip writer for fixtures only.
    fn zip(rows: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, data, deflate) in rows {
            let mut crc = flate2::Crc::new();
            crc.update(data);
            let body = if *deflate {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(data).unwrap();
                encoder.finish().unwrap()
            } else {
                data.to_vec()
            };
            let method: u16 = if *deflate { 8 } else { 0 };
            let offset = out.len() as u32;
            let mut common = Vec::new();
            common.extend_from_slice(&20_u16.to_le_bytes());
            common.extend_from_slice(&0_u16.to_le_bytes());
            common.extend_from_slice(&method.to_le_bytes());
            common.extend_from_slice(&[0; 4]);
            common.extend_from_slice(&crc.sum().to_le_bytes());
            common.extend_from_slice(&(body.len() as u32).to_le_bytes());
            common.extend_from_slice(&(data.len() as u32).to_le_bytes());
            common.extend_from_slice(&(name.len() as u16).to_le_bytes());
            common.extend_from_slice(&0_u16.to_le_bytes());
            out.extend_from_slice(&LOCAL.to_le_bytes());
            out.extend_from_slice(&common);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&body);
            directory.extend_from_slice(&CENTRAL.to_le_bytes());
            directory.extend_from_slice(&20_u16.to_le_bytes());
            directory.extend_from_slice(&common);
            // Comment length, disk start, internal attributes; external attributes.
            directory.extend_from_slice(&[0; 6]);
            directory.extend_from_slice(&0_u32.to_le_bytes());
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name.as_bytes());
        }
        let start = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(&EOCD.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(rows.len() as u16).to_le_bytes());
        out.extend_from_slice(&(rows.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out
    }
    fn scratch(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("hm-zip-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("out")).unwrap();
        dir
    }
    #[test]
    fn extracts_stored_and_deflate_members_with_crc() {
        let dir = scratch("good");
        let bytes = zip(&[
            ("sdk/", b"", false),
            ("dotnet.exe", b"muxer", false),
            ("sdk/10.0.403/x.dll", &[7_u8; 5000], true),
        ]);
        fs::write(dir.join("a.zip"), &bytes).unwrap();
        assert_eq!(extract(&dir.join("a.zip"), &dir.join("out")).unwrap(), 2);
        assert_eq!(fs::read(dir.join("out/dotnet.exe")).unwrap(), b"muxer");
        assert_eq!(
            fs::read(dir.join("out/sdk/10.0.403/x.dll")).unwrap(),
            [7; 5000]
        );
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn refuses_escapes_duplicates_and_corruption() {
        for name in ["../x", "/abs", "C:/x", "a\\b", "a/../b", "./x"] {
            assert!(!plain_name(name), "{name}");
        }
        assert!(plain_name("shared/Microsoft.NETCore.App/10.0.14/x.dll"));
        for (label, rows) in [
            ("escape", vec![("../x", b"x".as_slice(), false)]),
            (
                "dup",
                vec![("a", b"x".as_slice(), false), ("A", b"y", false)],
            ),
        ] {
            let dir = scratch(label);
            fs::write(dir.join("a.zip"), zip(&rows)).unwrap();
            assert!(
                extract(&dir.join("a.zip"), &dir.join("out")).is_err(),
                "{label}"
            );
            fs::remove_dir_all(&dir).unwrap();
        }
        let dir = scratch("crc");
        let mut bytes = zip(&[("a", b"hello", false)]);
        // Flip one data byte after the 30-byte local header and 1-byte name.
        bytes[31] ^= 1;
        fs::write(dir.join("a.zip"), &bytes).unwrap();
        assert!(extract(&dir.join("a.zip"), &dir.join("out")).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
