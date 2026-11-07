//! Private Linux diagnostic backend under quiescent cooperative ownership.
//! Metadata observations are not an atomic snapshot or hostile-writer defense.
use super::{Source, sealed};
use crate::{
    keys::custody::{Owner, permissions},
    storage::ExclusiveFile,
    verification::scan::limits::{Control, Limits, invalid, resource},
};
use rustix::fs::{AtFlags, Mode, OFlags, RawDir, ResolveFlags, StatxFlags, openat2};
use std::{
    fs::File,
    io::{self, BufRead, Read},
    mem::MaybeUninit,
    os::unix::{
        ffi::OsStrExt,
        fs::{FileExt, MetadataExt},
    },
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const SIDECAR: &str = "checkpoints.jsonl";
const LOCK: &str = ".audit-writer.lock";
const BUFFER: usize = 8192;

struct Admission {
    max_entries: usize,
    max_handles: usize,
    other_handles: usize,
}
impl Admission {
    fn check(&self, segments: usize) -> io::Result<()> {
        let peak = segments
            .checked_add(7)
            .and_then(|value| value.checked_add(self.other_handles))
            .ok_or_else(|| resource("native descriptor budget overflow"))?;
        if segments > 4096
            || self.max_entries == 0
            || self.max_entries > 4100
            || self.max_handles == 0
            || self.max_handles == usize::MAX
            || self.other_handles == usize::MAX
            || peak > self.max_handles
        {
            return Err(resource("native source descriptor admission refused"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Role {
    Parent,
    Directory,
    Payload,
}

#[derive(Debug, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    filesystem: i64,
    mount: u64,
}
struct Pin {
    file: File,
    stamp: Stamp,
    role: Role,
}
impl Pin {
    fn capture(file: File, role: Role) -> io::Result<Self> {
        let stamp = observe(&file, role)?;
        Ok(Self { file, stamp, role })
    }
    fn unchanged(&self) -> io::Result<()> {
        if observe(&self.file, self.role)? != self.stamp {
            return Err(invalid("native audit opened object changed"));
        }
        Ok(())
    }
    fn named(&self, file: File) -> io::Result<()> {
        if observe(&file, self.role)? != self.stamp {
            return Err(invalid("native audit named object changed"));
        }
        Ok(())
    }
}

fn observe(file: &File, role: Role) -> io::Result<Stamp> {
    let metadata = file.metadata()?;
    match role {
        Role::Parent if !metadata.is_dir() => {
            return Err(invalid("native parent is not directory"));
        }
        Role::Directory => permissions::directory_file(file, &Owner::CurrentUser)?,
        Role::Payload => permissions::file(file, &Owner::CurrentUser)?,
        Role::Parent => {}
    }
    let filesystem = rustix::fs::fstatfs(file)?.f_type;
    if !matches!(role, Role::Parent) && filesystem != 0xef53 {
        return Err(unsupported(
            "native audit requires observed ext-family filesystem",
        ));
    }
    let mount =
        rustix::fs::statx(file, "", AtFlags::EMPTY_PATH, StatxFlags::MNT_ID).map_err(api_error)?;
    if mount.stx_mask & StatxFlags::MNT_ID.bits() == 0 {
        return Err(unsupported("same-object mount identity unavailable"));
    }
    Ok(Stamp {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: if matches!(role, Role::Parent) {
            0
        } else {
            metadata.len()
        },
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        // Unrelated siblings may change the trusted parent's directory epoch.
        // Its identity/owner/mode and the root's exact current name remain checked.
        links: if matches!(role, Role::Parent) {
            0
        } else {
            metadata.nlink()
        },
        modified: if matches!(role, Role::Parent) {
            (0, 0)
        } else {
            (metadata.mtime(), metadata.mtime_nsec())
        },
        changed: if matches!(role, Role::Parent) {
            (0, 0)
        } else {
            (metadata.ctime(), metadata.ctime_nsec())
        },
        filesystem,
        mount: mount.stx_mnt_id,
    })
}

fn unsupported(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}
fn api_error(error: rustix::io::Errno) -> io::Error {
    if matches!(
        error,
        rustix::io::Errno::NOSYS | rustix::io::Errno::OPNOTSUPP | rustix::io::Errno::INVAL
    ) {
        unsupported("required native source API unavailable")
    } else {
        error.into()
    }
}
fn flags(directory: bool) -> OFlags {
    let base = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK;
    if directory {
        base | OFlags::DIRECTORY
    } else {
        base
    }
}
fn absolute(path: &Path) -> io::Result<File> {
    let fd = openat2(
        rustix::fs::CWD,
        path,
        flags(true),
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(api_error)?;
    Ok(File::from(fd))
}
fn relative(parent: &File, name: &str, directory: bool, cross_mount: bool) -> io::Result<File> {
    let mut resolve =
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS;
    if !cross_mount {
        resolve |= ResolveFlags::NO_XDEV;
    }
    Ok(File::from(
        openat2(parent, name, flags(directory), Mode::empty(), resolve).map_err(api_error)?,
    ))
}

struct Context<'scan, 'cancel> {
    control: &'scan Control<'cancel>,
    limits: &'scan Limits,
    admission: Admission,
    path: PathBuf,
    root_name: String,
    parent: Pin,
    root: Pin,
    lock_observer: Pin,
    segments: Vec<Pin>,
    sidecar: Option<Pin>,
    poisoned: AtomicBool,
    _guard: ExclusiveFile,
}
struct NativeSource<'scan, 'cancel> {
    context: Arc<Context<'scan, 'cancel>>,
}

impl<'scan, 'cancel> NativeSource<'scan, 'cancel> {
    fn open(
        path: &Path,
        limits: &'scan Limits,
        control: &'scan Control<'cancel>,
        admission: Admission,
    ) -> io::Result<Self> {
        limits.validate()?;
        control.check()?;
        // Conservative N=max_segments, P=1 admission occurs before ANY native open.
        admission.check(limits.segments)?;
        if !path.is_absolute()
            || path.as_os_str().as_bytes().len() > 4096
            || path
                .as_os_str()
                .as_bytes()
                .split(|byte| *byte == b'/')
                .any(|part| matches!(part, b"." | b".."))
            || path
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        {
            return Err(invalid(
                "native source requires bounded absolute unaliased path",
            ));
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| invalid("native root name unavailable"))?;
        let parent_path = path
            .parent()
            .ok_or_else(|| invalid("native root parent unavailable"))?;
        let parent = Pin::capture(absolute(parent_path)?, Role::Parent)?;
        control.check()?;
        let root = Pin::capture(relative(&parent.file, name, true, true)?, Role::Directory)?;
        control.check()?;
        let lock_observer = Pin::capture(relative(&root.file, LOCK, false, false)?, Role::Payload)?;
        same_mount(&root, &lock_observer)?;
        let guard = ExclusiveFile::acquire(lock_observer.file.try_clone()?)?;
        let mut context = Context {
            control,
            limits,
            admission,
            path: path.to_path_buf(),
            root_name: name.to_owned(),
            parent,
            root,
            lock_observer,
            segments: Vec::new(),
            sidecar: None,
            poisoned: AtomicBool::new(false),
            _guard: guard,
        };
        let (count, has_sidecar) = context.membership(None)?;
        context
            .segments
            .try_reserve_exact(count)
            .map_err(|_| io::Error::other("native pin inventory allocation refused"))?;
        let mut bytes = 0u64;
        for index in 1..=count {
            control.check()?;
            let pin = Pin::capture(
                relative(&context.root.file, &segment_name(index), false, false)?,
                Role::Payload,
            )?;
            same_mount(&context.root, &pin)?;
            bytes = bytes
                .checked_add(pin.stamp.length)
                .filter(|bytes| *bytes <= limits.record_bytes)
                .ok_or_else(|| resource("native record bytes exceeded"))?;
            context.segments.push(pin);
        }
        if has_sidecar {
            control.check()?;
            let pin = Pin::capture(
                relative(&context.root.file, SIDECAR, false, false)?,
                Role::Payload,
            )?;
            same_mount(&context.root, &pin)?;
            if pin.stamp.length > limits.checkpoint_bytes {
                return Err(resource("native sidecar bytes exceeded"));
            }
            context.sidecar = Some(pin);
        }
        let source = Self {
            context: Arc::new(context),
        };
        source.unchanged()?;
        Ok(source)
    }
    fn reader(&self, index: Option<usize>) -> io::Result<NativeReader<'scan, 'cancel>> {
        self.context.ready()?;
        let mut buffer = Vec::new();
        buffer.try_reserve_exact(BUFFER).map_err(|_| {
            self.context.poisoned.store(true, Ordering::Release);
            io::Error::other("native reader allocation refused")
        })?;
        buffer.resize(BUFFER, 0);
        Ok(NativeReader {
            context: Arc::clone(&self.context),
            index,
            buffer,
            offset: 0,
            position: 0,
            available: 0,
            failed: false,
        })
    }
}
fn segment_name(index: usize) -> String {
    format!("audit-{index:010}.log")
}
fn same_mount(root: &Pin, pin: &Pin) -> io::Result<()> {
    if root.stamp.device != pin.stamp.device || root.stamp.mount != pin.stamp.mount {
        return Err(invalid("native audit member mount differs"));
    }
    Ok(())
}

impl Context<'_, '_> {
    fn ready(&self) -> io::Result<()> {
        if self.poisoned.load(Ordering::Acquire) {
            return Err(invalid("native audit source poisoned"));
        }
        let result = self.control.check();
        if result.is_err() {
            self.poisoned.store(true, Ordering::Release);
        }
        result
    }
    fn membership(&self, expected: Option<usize>) -> io::Result<(usize, bool)> {
        self.ready()?;
        let descriptor = relative(&self.root.file, ".", true, false)?;
        let mut buffer = [MaybeUninit::uninit(); 4096];
        let mut directory = RawDir::new(descriptor, &mut buffer);
        let mut seen = Vec::new();
        seen.try_reserve_exact(self.limits.segments)
            .map_err(|_| io::Error::other("native membership allocation refused"))?;
        seen.resize(self.limits.segments, false);
        let (mut entries, mut count, mut sidecar, mut lock) = (0usize, 0usize, false, false);
        while let Some(entry) = directory.next() {
            self.ready()?;
            entries = entries
                .checked_add(1)
                .filter(|count| *count <= self.admission.max_entries)
                .ok_or_else(|| resource("native directory entry bound exceeded"))?;
            let entry = entry?;
            let name = entry.file_name().to_bytes();
            if matches!(name, b"." | b"..") {
                continue;
            }
            let name = std::str::from_utf8(name)
                .map_err(|_| invalid("native noncanonical member name"))?;
            let pin = if name == LOCK {
                if lock {
                    return Err(invalid("duplicate native lock name"));
                }
                lock = true;
                Some(&self.lock_observer)
            } else if name == SIDECAR {
                if sidecar {
                    return Err(invalid("duplicate native sidecar name"));
                }
                sidecar = true;
                self.sidecar.as_ref()
            } else {
                let number = name
                    .strip_prefix("audit-")
                    .and_then(|s| s.strip_suffix(".log"))
                    .filter(|s| s.len() == 10 && s.bytes().all(|b| b.is_ascii_digit()))
                    .ok_or_else(|| invalid("native unknown directory member"))?;
                let index = number
                    .parse::<usize>()
                    .map_err(|_| invalid("native segment index invalid"))?;
                if index == 0 || index > seen.len() || seen[index - 1] {
                    return Err(invalid("native segment index duplicate or outside bound"));
                }
                seen[index - 1] = true;
                count += 1;
                self.segments.get(index - 1)
            };
            let named = relative(&self.root.file, name, false, false)?;
            if let Some(pin) = pin {
                pin.named(named)?;
            } else {
                let pin = Pin::capture(named, Role::Payload)?;
                same_mount(&self.root, &pin)?;
            }
        }
        self.ready()?;
        if !lock
            || seen[..count].iter().any(|seen| !seen)
            || expected
                .is_some_and(|expected| expected != count || sidecar != self.sidecar.is_some())
        {
            return Err(invalid("native audit membership changed or gapped"));
        }
        Ok((count, sidecar))
    }
    fn validate(&self) -> io::Result<()> {
        self.ready()?;
        self.parent.unchanged()?;
        self.root.unchanged()?;
        self.root
            .named(relative(&self.parent.file, &self.root_name, true, true)?)?;
        self.root.named(absolute(&self.path)?)?;
        self.lock_observer.unchanged()?;
        for pin in &self.segments {
            self.ready()?;
            pin.unchanged()?;
        }
        if let Some(pin) = &self.sidecar {
            self.ready()?;
            pin.unchanged()?;
        }
        self.membership(Some(self.segments.len()))?;
        self.root.unchanged()?;
        self.parent.unchanged()?;
        self.ready()
    }
}
impl sealed::Sealed for NativeSource<'_, '_> {}
impl<'scan, 'cancel> Source for NativeSource<'scan, 'cancel> {
    type Reader = NativeReader<'scan, 'cancel>;
    fn segment_count(&self) -> io::Result<usize> {
        self.context.ready()?;
        Ok(self.context.segments.len())
    }
    fn segment(&self, index: usize) -> io::Result<Self::Reader> {
        if index == 0 || index > self.context.segments.len() {
            self.context.poisoned.store(true, Ordering::Release);
            return Err(invalid("native segment cursor outside inventory"));
        }
        self.reader(Some(index - 1))
    }
    fn checkpoints(&self) -> io::Result<Option<Self::Reader>> {
        self.context.ready()?;
        self.context
            .sidecar
            .as_ref()
            .map(|_| self.reader(None))
            .transpose()
    }
    fn unchanged(&self) -> io::Result<()> {
        let result = self.context.validate();
        if result.is_err() {
            self.context.poisoned.store(true, Ordering::Release);
        }
        result
    }
}

struct NativeReader<'scan, 'cancel> {
    context: Arc<Context<'scan, 'cancel>>,
    index: Option<usize>,
    buffer: Vec<u8>,
    offset: u64,
    position: usize,
    available: usize,
    failed: bool,
}
impl NativeReader<'_, '_> {
    fn fill(&mut self) -> io::Result<()> {
        self.context.ready()?;
        if self.position < self.available {
            return Ok(());
        }
        let pin = match self.index {
            Some(index) => &self.context.segments[index],
            None => self
                .context
                .sidecar
                .as_ref()
                .ok_or_else(|| invalid("native sidecar cursor missing"))?,
        };
        pin.unchanged()?;
        let remaining = pin
            .stamp
            .length
            .checked_sub(self.offset)
            .ok_or_else(|| invalid("native reader offset exceeds captured end"))?;
        let length = usize::try_from(remaining.min(self.buffer.len() as u64))
            .map_err(|_| resource("native read size overflow"))?;
        self.position = 0;
        self.available = 0;
        if length == 0 {
            return Ok(());
        }
        loop {
            self.context.ready()?;
            match pin.file.read_at(&mut self.buffer[..length], self.offset) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "native file truncated before captured end",
                    ));
                }
                Ok(count) => {
                    self.available = count;
                    break;
                }
            }
        }
        pin.unchanged()?;
        self.context.ready()
    }
}
impl BufRead for NativeReader<'_, '_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.failed {
            return Err(invalid("native reader poisoned"));
        }
        self.failed = true;
        if let Err(error) = self.fill() {
            self.context.poisoned.store(true, Ordering::Release);
            return Err(error);
        }
        self.failed = false;
        Ok(&self.buffer[self.position..self.available])
    }
    fn consume(&mut self, amount: usize) {
        if amount > self.available - self.position {
            self.failed = true;
            self.context.poisoned.store(true, Ordering::Release);
            return;
        }
        if let Some(next) = self.offset.checked_add(amount as u64) {
            self.offset = next;
            self.position += amount;
        } else {
            self.failed = true;
            self.context.poisoned.store(true, Ordering::Release);
        }
    }
}
impl Read for NativeReader<'_, '_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let bytes = self.fill_buf()?;
        let count = bytes.len().min(out.len());
        out[..count].copy_from_slice(&bytes[..count]);
        self.consume(count);
        Ok(count)
    }
}

#[cfg(test)]
mod tests;
