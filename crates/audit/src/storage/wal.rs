use super::{framing, recover_segment};
use crate::{checkpoint::Checkpoint, record::Record};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
};

/// Explicit disk and parsing bounds. Frame size includes its newline.
pub struct WalOptions {
    /// Rotate before the next append after this many bytes are committed.
    pub rotate_bytes: u64,
    /// Maximum serialized record/checkpoint size, including framing.
    pub max_frame: usize,
}
impl Default for WalOptions {
    fn default() -> Self {
        Self {
            rotate_bytes: 16 << 20,
            max_frame: 4 << 20,
        }
    }
}

/// Single-writer fsync-backed JSONL storage, not a chain authentication service.
/// The directory must be service-owned with restrictive native permissions.
/// Cooperative file locking does not protect against a hostile directory owner.
pub struct Wal {
    dir: PathBuf,
    options: WalOptions,
    _lock: File,
    segment: File,
    index: u64,
    written: u64,
    poisoned: bool,
    recovered_bytes: u64,
}

impl Wal {
    /// Authenticate a consistent disk snapshot while retaining the OS writer
    /// lock. The history is not trusted merely because JSON parsing succeeded.
    pub fn verified_history(
        &self,
        policy: super::RecoveryPolicy<'_>,
    ) -> io::Result<super::VerifiedHistory> {
        super::verify_history(self.records()?, self.checkpoints()?, policy).map_err(invalid)
    }
    /// Acquire the OS writer lock, validate all frames and repair only a torn
    /// terminal record. Malformed checkpoints prevent opening. Cryptographic
    /// verification is still required before restoring trusted chain state.
    pub fn open(dir: &Path, options: WalOptions) -> io::Result<Self> {
        if options.rotate_bytes == 0 || options.max_frame == 0 {
            return Err(invalid("zero WAL bound"));
        }
        fs::create_dir_all(dir)?;
        if !fs::symlink_metadata(dir)?.file_type().is_dir() {
            return Err(invalid("WAL directory is not a real directory"));
        }
        let lock = open_append(&dir.join(".audit-writer.lock"))?;
        fs4::FileExt::try_lock(&lock).map_err(io::Error::from)?;
        let segments = segments(dir)?;
        let mut recovered_bytes = 0;
        for (position, (_, path)) in segments.iter().enumerate() {
            recovered_bytes +=
                recover_segment(path, position + 1 == segments.len(), options.max_frame)?
                    .truncated_bytes;
        }
        read_checkpoints(dir, options.max_frame)?;
        let index = segments.last().map_or(1, |(index, _)| *index);
        let segment = open_append(&segment_path(dir, index))?;
        sync_directory(dir)?;
        let written = segment.metadata()?.len();
        Ok(Self {
            dir: dir.into(),
            options,
            _lock: lock,
            segment,
            index,
            written,
            poisoned: false,
            recovered_bytes,
        })
    }

    /// Bytes discarded from a torn terminal record during this open. Emit an
    /// operational recovery notice without treating discarded bytes as events.
    pub fn recovered_bytes(&self) -> u64 {
        self.recovered_bytes
    }

    /// Append an already sealed record. Success means write_all and sync_all
    /// succeeded; callers may release enforcement only after this boundary.
    /// Any disk failure poisons this instance until a verified restart.
    pub fn append_record(&mut self, record: &Record) -> io::Result<()> {
        self.healthy()?;
        let line = self.encode(record)?;
        let result = (|| {
            if self.written >= self.options.rotate_bytes {
                self.segment.sync_all()?;
                let next = self
                    .index
                    .checked_add(1)
                    .ok_or_else(|| invalid("segment index overflow"))?;
                if next > 9_999_999_999 {
                    return Err(invalid("segment filename index exhausted"));
                }
                let path = segment_path(&self.dir, next);
                let segment = OpenOptions::new().write(true).create_new(true).open(path)?;
                sync_directory(&self.dir)?;
                self.segment = segment;
                self.index = next;
                self.written = 0;
            }
            self.segment.write_all(&line)?;
            self.segment.sync_all()?;
            self.written += line.len() as u64;
            Ok(())
        })();
        self.finish(result)
    }

    /// Persist a signed checkpoint with the same fsync-before-success boundary.
    pub fn append_checkpoint(&mut self, checkpoint: &Checkpoint) -> io::Result<()> {
        self.healthy()?;
        let line = self.encode(checkpoint)?;
        let result = (|| {
            let mut file = open_append(&self.dir.join("checkpoints.jsonl"))?;
            file.write_all(&line)?;
            file.sync_all()?;
            sync_directory(&self.dir)
        })();
        self.finish(result)
    }

    /// Read committed records under this writer's exclusive ownership.
    pub fn records(&self) -> io::Result<Vec<Record>> {
        self.healthy()?;
        let mut records = Vec::new();
        for (_, path) in segments(&self.dir)? {
            let file = File::open(path)?;
            records.extend(
                framing::decode::<Record>(BufReader::new(file), self.options.max_frame, false)?
                    .values,
            );
        }
        Ok(records)
    }

    /// Read framed checkpoints; absence is an empty chain, corruption is fatal.
    pub fn checkpoints(&self) -> io::Result<Vec<Checkpoint>> {
        self.healthy()?;
        read_checkpoints(&self.dir, self.options.max_frame)
    }

    /// Explicit shutdown durability check; Drop cannot report an I/O failure.
    pub fn flush(&mut self) -> io::Result<()> {
        self.healthy()?;
        let result = self.segment.sync_all();
        self.finish(result)
    }

    fn healthy(&self) -> io::Result<()> {
        if self.poisoned {
            Err(io::Error::other("WAL poisoned by an earlier disk failure"))
        } else {
            Ok(())
        }
    }
    fn finish(&mut self, result: io::Result<()>) -> io::Result<()> {
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn encode(&self, value: &impl serde::Serialize) -> io::Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec(value).map_err(invalid)?;
        bytes.push(b'\n');
        if bytes.len() > self.options.max_frame {
            return Err(invalid("WAL frame exceeds configured limit"));
        }
        Ok(bytes)
    }
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
fn segment_path(dir: &Path, index: u64) -> PathBuf {
    dir.join(format!("audit-{index:010}.log"))
}
fn open_append(path: &Path) -> io::Result<File> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(invalid("WAL entry must be a regular file"));
        }
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(path)
}
fn segments(dir: &Path) -> io::Result<Vec<(u64, PathBuf)>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("audit-") && name.ends_with(".log") {
            let number = &name[6..name.len() - 4];
            if number.len() != 10
                || !number.bytes().all(|b| b.is_ascii_digit())
                || !entry.file_type()?.is_file()
            {
                return Err(invalid("invalid audit segment entry"));
            }
            let index = number.parse::<u64>().map_err(invalid)?;
            result.push((index, entry.path()));
        }
    }
    result.sort_by_key(|(index, _)| *index);
    for (position, (index, _)) in result.iter().enumerate() {
        if *index != position as u64 + 1 {
            return Err(invalid("audit segment gap"));
        }
    }
    Ok(result)
}
fn read_checkpoints(dir: &Path, max_frame: usize) -> io::Result<Vec<Checkpoint>> {
    let path = dir.join("checkpoints.jsonl");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(invalid("checkpoint sidecar must be a regular file"));
        }
        _ => {}
    }
    Ok(framing::decode(BufReader::new(File::open(path)?), max_frame, false)?.values)
}
fn sync_directory(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir; // Windows directory persistence requires native deployment validation.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "humanymous-wal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }
    fn options() -> WalOptions {
        WalOptions {
            rotate_bytes: 1,
            max_frame: 4096,
        }
    }
    #[test]
    fn exclusive_writer_rotation_checkpoint_and_restart() {
        let dir = directory();
        let mut wal = Wal::open(&dir, options()).unwrap();
        assert_eq!(
            Wal::open(&dir, options()).err().unwrap().kind(),
            io::ErrorKind::WouldBlock
        );
        for seq in 1..=3 {
            wal.append_record(&Record {
                seq,
                ..Record::default()
            })
            .unwrap();
        }
        let cp = Checkpoint {
            tree_size: 3,
            ..Checkpoint::default()
        };
        wal.append_checkpoint(&cp).unwrap();
        wal.flush().unwrap();
        assert_eq!(segments(&dir).unwrap().len(), 3);
        assert_eq!(
            wal.records()
                .unwrap()
                .iter()
                .map(|r| r.seq)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        drop(wal);
        let reopened = Wal::open(&dir, options()).unwrap();
        assert_eq!(reopened.records().unwrap().len(), 3);
        assert_eq!(reopened.checkpoints().unwrap(), vec![cp]);
        drop(reopened);
        for entry in fs::read_dir(&dir).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn rotation_failure_permanently_poisoned_until_restart() {
        let dir = directory();
        let mut wal = Wal::open(&dir, options()).unwrap();
        wal.append_record(&Record::default()).unwrap();
        let obstacle = segment_path(&dir, 2);
        fs::write(&obstacle, b"reserved").unwrap();
        assert!(wal.append_record(&Record::default()).is_err());
        fs::remove_file(obstacle).unwrap();
        assert!(wal.append_record(&Record::default()).is_err());
        assert!(wal.append_checkpoint(&Checkpoint::default()).is_err());
        assert!(wal.records().is_err());
        drop(wal);
        assert_eq!(
            Wal::open(&dir, options()).unwrap().records().unwrap().len(),
            1
        );
        for entry in fs::read_dir(&dir).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn malformed_sidecar_and_segment_gap_fail_closed() {
        let dir = directory();
        fs::write(dir.join("checkpoints.jsonl"), b"{}").unwrap();
        assert!(Wal::open(&dir, options()).is_err());
        fs::remove_file(dir.join("checkpoints.jsonl")).unwrap();
        fs::write(segment_path(&dir, 2), b"{}\n").unwrap();
        assert!(Wal::open(&dir, options()).is_err());
        for entry in fs::read_dir(&dir).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
}
