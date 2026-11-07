//! Only scanner-owned backends may supply readers or source-epoch observations.
use std::{
    io::{self, BufRead},
    path::Path,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod native;

pub(super) mod sealed {
    pub trait Sealed {}
}
pub(super) trait Source: sealed::Sealed {
    type Reader: BufRead;
    fn segment_count(&self) -> io::Result<usize>;
    fn segment(&self, index: usize) -> io::Result<Self::Reader>;
    fn checkpoints(&self) -> io::Result<Option<Self::Reader>>;
    fn unchanged(&self) -> io::Result<()>;
}

// No supported native producer exists until opened-object/provider identity is
// accepted. This uninhabited type cannot become a caller-supplied trust token.
pub(super) enum Unavailable {}
pub(super) fn open_native(_: &Path) -> io::Result<Unavailable> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native audit source identity unavailable",
    ))
}
