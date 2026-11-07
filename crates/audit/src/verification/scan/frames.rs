use super::limits::{Control, Limits, invalid, resource};
use crate::storage::framing::{
    cursor::{FrameCursor, content},
    preflight,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::io::{self, BufRead};

pub(super) struct Budget {
    pub bytes: u64,
    pub values: usize,
    pub digest: Sha256,
    max_bytes: u64,
    max_values: usize,
}
impl Budget {
    pub fn new(max_bytes: u64, max_values: usize) -> Self {
        Self {
            bytes: 0,
            values: 0,
            digest: Sha256::new(),
            max_bytes,
            max_values,
        }
    }
    fn bytes(&mut self, raw: &[u8]) -> io::Result<()> {
        let count = u64::try_from(raw.len()).map_err(|_| resource("scan byte counter overflow"))?;
        let next = self
            .bytes
            .checked_add(count)
            .filter(|next| *next <= self.max_bytes)
            .ok_or_else(|| resource("scan byte limit exceeded"))?;
        self.digest.update(raw);
        self.bytes = next;
        Ok(())
    }

    pub fn begin_segment(&mut self, index: usize) -> io::Result<()> {
        self.digest.update(b"audit-record-segment-v1\0");
        self.digest.update(
            u64::try_from(index)
                .map_err(|_| resource("audit segment counter overflow"))?
                .to_be_bytes(),
        );
        Ok(())
    }

    pub fn end_segment(&mut self, end: &End) -> io::Result<()> {
        self.digest.update(end.complete.to_be_bytes());
        self.digest.update(
            u64::try_from(end.torn)
                .map_err(|_| resource("audit torn byte overflow"))?
                .to_be_bytes(),
        );
        Ok(())
    }

    pub fn sidecar(&mut self, present: bool) {
        self.digest.update(b"audit-checkpoint-sidecar-v1\0");
        self.digest.update([u8::from(present)]);
    }

    pub fn signature(&self) -> [u8; 32] {
        self.digest.clone().finalize().into()
    }
}

pub(super) struct Frames<R> {
    cursor: FrameCursor<R>,
    done: bool,
    failed: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct End {
    pub complete: u64,
    pub torn: usize,
}
impl<R: BufRead> Frames<R> {
    pub fn new(reader: R, frame: usize, allow_tail: bool) -> io::Result<Self> {
        Ok(Self {
            cursor: FrameCursor::new(reader, frame, allow_tail)?,
            done: false,
            failed: false,
        })
    }
    pub fn next<T: DeserializeOwned>(
        &mut self,
        budget: &mut Budget,
        limits: &Limits,
        control: &Control<'_>,
    ) -> io::Result<Option<T>> {
        if self.failed {
            return Err(invalid("bounded audit frame stream failed"));
        }
        if self.done {
            return Ok(None);
        }
        self.failed = true;
        loop {
            control.check()?;
            let Some(raw) = self.cursor.next_raw_frame()? else {
                control.check()?;
                budget.bytes(self.cursor.raw_tail())?;
                control.check()?;
                self.done = true;
                self.failed = false;
                return Ok(None);
            };
            control.check()?;
            budget.bytes(raw)?;
            let bytes = content(raw);
            if bytes.is_empty() {
                continue;
            }
            let next = budget
                .values
                .checked_add(1)
                .filter(|next| *next <= budget.max_values)
                .ok_or_else(|| resource("scan value count exceeded"))?;
            preflight::check(bytes, limits.json)
                .map_err(|_| resource("scan JSON budget exceeded"))?;
            control.check()?;
            let value =
                serde_json::from_slice(bytes).map_err(|_| invalid("invalid audit JSON frame"))?;
            control.check()?;
            budget.values = next;
            self.failed = false;
            return Ok(Some(value));
        }
    }
    pub fn finish(self) -> io::Result<End> {
        if self.failed || !self.done {
            return Err(invalid("audit frame stream did not reach EOF"));
        }
        Ok(End {
            complete: self.cursor.complete_bytes(),
            torn: self.cursor.raw_tail().len(),
        })
    }
}
