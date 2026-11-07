use super::*;
use std::{
    io::{Read, Write},
    sync::mpsc::{RecvTimeoutError, TryRecvError},
    time::Duration,
};

pub(super) fn output(mut pipe: impl Read + Send + 'static, limit: usize) -> io::Result<Output> {
    let (send, chunks) = mpsc::sync_channel(OUTPUT_SLOTS);
    let worker = Worker::start("interactive-output", move || {
        read_output(&mut pipe, limit, send)
    })?;
    Ok(Output { worker, chunks })
}
// Worker result has its OWN fixed one-slot channel, independent of payload slots.
pub(super) fn read_output(
    pipe: &mut impl Read,
    limit: usize,
    send: SyncSender<OutputChunk>,
) -> io::Result<()> {
    let mut total = 0usize;
    loop {
        let mut storage = vec![0u8; CHUNK].into_boxed_slice();
        let want = CHUNK.min(limit - total + 1);
        let used = match pipe.read(&mut storage[..want]) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        total += used;
        if total > limit {
            return Err(io::Error::other("interactive output lifetime cap exceeded"));
        }
        send.try_send(OutputChunk { storage, used })
            .map_err(|_| io::Error::other("interactive output queue unavailable"))?;
    }
}
pub(super) fn input(
    mut pipe: impl Write,
    startup: Receiver<Box<[u8]>>,
    controls: Receiver<Box<[u8]>>,
    cancel: Arc<AtomicBool>,
    written: Arc<AtomicUsize>,
    until: Instant,
) -> io::Result<()> {
    let check = || {
        if cancel.load(Ordering::Acquire) || Instant::now() >= until {
            Err(io::Error::other("interactive input cancelled or expired"))
        } else {
            Ok(())
        }
    };
    let first = loop {
        check()?;
        match startup.recv_timeout(Duration::from_millis(1)) {
            Ok(bytes) => break bytes,
            Err(RecvTimeoutError::Timeout) => (),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("interactive startup missing"));
            }
        }
    };
    write_message(&mut pipe, first, &written, &check)?;
    loop {
        check()?;
        match controls.try_recv() {
            Ok(bytes) => write_message(&mut pipe, bytes, &written, &check)?,
            Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}
fn write_message(
    pipe: &mut impl Write,
    bytes: Box<[u8]>,
    written: &AtomicUsize,
    check: &impl Fn() -> io::Result<()>,
) -> io::Result<()> {
    check()?;
    pipe.write_all(&bytes)?;
    pipe.flush()?;
    written
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
        .map_err(|_| io::Error::other("interactive message counter exhausted"))?;
    check()
}
