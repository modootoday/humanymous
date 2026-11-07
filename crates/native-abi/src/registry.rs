//! One owner of all allocation and context lifetimes.
use crate::{Status, Version};
use std::collections::{BTreeMap, BTreeSet};
use zeroize::Zeroizing;

/// Maximum simultaneously live contexts in one process-lifetime registry.
pub const MAX_CONTEXTS: usize = 16;
/// Maximum buffers per live context.
pub const MAX_BUFFERS: usize = 64;
/// Maximum initialized bytes in a buffer.
pub const MAX_BUFFER_BYTES: u64 = 1_048_576;
/// Maximum live initialized bytes across all contexts.
pub const MAX_TOTAL_BYTES: u64 = 8_388_608;

struct Buffer {
    owner: u64,
    bytes: Zeroizing<Vec<u8>>,
}

/// Safe, exclusively borrowed process-local resource registry.
///
/// A future export facade must keep one registry for the library's process
/// lifetime. Recreating it would reset the handle namespace. Handles are not
/// authenticated capabilities against malicious code in the same process.
pub struct Registry {
    next: u64,
    contexts: BTreeSet<u64>,
    buffers: BTreeMap<u64, Buffer>,
    total: u64,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            next: 1,
            contexts: BTreeSet::new(),
            buffers: BTreeMap::new(),
            total: 0,
        }
    }
}
impl Registry {
    pub(crate) fn revoke_all(&mut self) {
        self.contexts.clear();
        self.buffers.clear();
        self.total = 0;
    }
    fn id(&mut self) -> Result<u64, Status> {
        let id = self.next;
        if id == 0 {
            return Err(Status::CAPACITY);
        }
        self.next = id.checked_add(1).unwrap_or(0);
        Ok(id)
    }
    /// Validate exact live context membership without minting another handle.
    ///
    /// Adapter-specific operations must check this before using their mapped
    /// request state. Liveness alone is not authentication or origin authority.
    pub fn validate_context(&self, context: u64) -> Result<(), Status> {
        if self.contexts.contains(&context) {
            Ok(())
        } else {
            Err(Status::HANDLE)
        }
    }
    fn buffer(&self, context: u64, handle: u64) -> Result<&Buffer, Status> {
        self.validate_context(context)?;
        self.buffers
            .get(&handle)
            .filter(|b| b.owner == context)
            .ok_or(Status::HANDLE)
    }
    /// Create an empty context after exact version negotiation.
    pub fn open(&mut self, version: Version) -> Result<u64, Status> {
        version.validate()?;
        if self.contexts.len() >= MAX_CONTEXTS {
            return Err(Status::CAPACITY);
        }
        let id = self.id()?;
        self.contexts.insert(id);
        Ok(id)
    }
    /// Revoke a context and zeroize/free every buffer it owns.
    ///
    /// No worker, callback or operating-system resource exists in this slice.
    pub fn close(&mut self, context: u64) -> Result<(), Status> {
        self.validate_context(context)?;
        self.contexts.remove(&context);
        self.buffers.retain(|_, buffer| {
            if buffer.owner == context {
                self.total -= buffer.bytes.len() as u64;
                false
            } else {
                true
            }
        });
        Ok(())
    }
    /// Allocate an initialized zero-filled, bounded, context-owned buffer.
    pub fn allocate(&mut self, context: u64, length: u64) -> Result<u64, Status> {
        self.validate_context(context)?;
        if length > MAX_BUFFER_BYTES {
            return Err(Status::ARGUMENT);
        }
        if self.buffers.values().filter(|b| b.owner == context).count() >= MAX_BUFFERS
            || self
                .total
                .checked_add(length)
                .is_none_or(|n| n > MAX_TOTAL_BYTES)
        {
            return Err(Status::CAPACITY);
        }
        let length = usize::try_from(length).map_err(|_| Status::ARGUMENT)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| Status::ALLOCATION)?;
        bytes.resize(length, 0);
        let bytes = Zeroizing::new(bytes);
        let id = self.id()?;
        self.buffers.insert(
            id,
            Buffer {
                owner: context,
                bytes,
            },
        );
        self.total += length as u64;
        Ok(id)
    }
    /// Return initialized length after validating exact ownership.
    pub fn length(&self, context: u64, handle: u64) -> Result<u64, Status> {
        Ok(self.buffer(context, handle)?.bytes.len() as u64)
    }
    /// Borrow the complete initialized buffer after exact ownership checks.
    ///
    /// No copy or allocation occurs. The Rust borrow prevents mutation/free of
    /// this registry while the view remains live. Adapter composition roots must
    /// not retain foreign pointers to it or treat bytes as authenticated facts.
    pub fn buffer_bytes(&self, context: u64, handle: u64) -> Result<&[u8], Status> {
        Ok(&self.buffer(context, handle)?.bytes)
    }
    /// Copy complete input or reject without modifying any byte.
    pub fn write(
        &mut self,
        context: u64,
        handle: u64,
        offset: u64,
        input: &[u8],
    ) -> Result<(), Status> {
        let buffer = self.buffer(context, handle)?;
        let range = range(offset, input.len(), buffer.bytes.len())?;
        self.buffers.get_mut(&handle).ok_or(Status::HANDLE)?.bytes[range].copy_from_slice(input);
        Ok(())
    }
    /// Copy initialized bytes into a caller-owned slice; retain no reference.
    pub fn read(
        &self,
        context: u64,
        handle: u64,
        offset: u64,
        output: &mut [u8],
    ) -> Result<(), Status> {
        let buffer = self.buffer(context, handle)?;
        let range = range(offset, output.len(), buffer.bytes.len())?;
        output.copy_from_slice(&buffer.bytes[range]);
        Ok(())
    }
    /// Zeroize then free the exact owned allocation. Double free rejects.
    pub fn free(&mut self, context: u64, handle: u64) -> Result<(), Status> {
        let length = self.length(context, handle)?;
        self.buffers.remove(&handle);
        self.total -= length;
        Ok(())
    }
    /// Live initialized bytes, for bounded non-sensitive accounting.
    pub fn allocated_bytes(&self) -> u64 {
        self.total
    }
}
fn range(offset: u64, length: usize, capacity: usize) -> Result<std::ops::Range<usize>, Status> {
    let start = usize::try_from(offset).map_err(|_| Status::ARGUMENT)?;
    let end = start
        .checked_add(length)
        .filter(|&n| n <= capacity)
        .ok_or(Status::ARGUMENT)?;
    Ok(start..end)
}

#[cfg(test)]
mod tests;
