//! Only scanner-owned backends may supply readers or source-epoch observations.
use std::{
    io::{self, BufRead},
    path::Path,
};

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod native;

pub(super) fn inspect_native(
    path: &Path,
    policy: &crate::storage::RecoveryPolicy<'_>,
    limits: &super::limits::Limits,
    control: &super::limits::Control<'_>,
    budget: super::NativeBudget,
) -> io::Result<super::Diagnostic> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        native::inspect_native(path, policy, limits, control, budget)
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (path, policy);
        limits.validate()?;
        control.check()?;
        budget.check(limits.segments)?;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native audit diagnostic profile unavailable",
        ))
    }
}

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

// The diagnostic facade does not promote this unavailable operational factory.
// This uninhabited type cannot become a caller-supplied ownership/trust token.
pub(super) enum Unavailable {}
pub(super) fn open_native(_: &Path) -> io::Result<Unavailable> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native audit source identity unavailable",
    ))
}
