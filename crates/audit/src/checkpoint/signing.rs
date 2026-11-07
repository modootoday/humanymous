//! Shared fixed-order encoder with checked count-and-replay allocation.
use super::Checkpoint;
use humanymous_signals::wire::quote_json_go_to;
use std::io::{self, Write};

fn budget() -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        "checkpoint signing resource limit exceeded",
    )
}

struct Limited<W> {
    inner: W,
    written: usize,
    limit: usize,
}
impl<W: Write> Write for Limited<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.written {
            return Err(budget());
        }
        let count = self.inner.write(bytes)?;
        self.written = self.written.checked_add(count).ok_or_else(budget)?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn encode(checkpoint: &Checkpoint, out: &mut impl Write) -> io::Result<()> {
    out.write_all(b"{\"node_id\":")?;
    quote_json_go_to(&checkpoint.node_id, out)?;
    write!(out, ",\"tree_size\":{},\"root\":", checkpoint.tree_size)?;
    quote_json_go_to(&checkpoint.root, out)?;
    out.write_all(b",\"merkle_root\":")?;
    quote_json_go_to(&checkpoint.merkle_root, out)?;
    out.write_all(b",\"prev_checkpoint_hash\":")?;
    quote_json_go_to(&checkpoint.prev_checkpoint_hash, out)?;
    out.write_all(b"}")
}

pub(super) fn bounded(checkpoint: &Checkpoint, limit: usize) -> io::Result<Vec<u8>> {
    let mut count = Limited {
        inner: io::sink(),
        written: 0,
        limit,
    };
    encode(checkpoint, &mut count)?;
    let expected = count.written;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected)
        .map_err(|_| io::Error::other("checkpoint signing allocation refused"))?;
    let mut replay = Limited {
        inner: &mut bytes,
        written: 0,
        limit: expected,
    };
    encode(checkpoint, &mut replay)?;
    if replay.written != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "checkpoint signing count and replay disagree",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
