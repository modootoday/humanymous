//! Fixed-width candidate wire layouts; not an exported ABI yet.

/// Exact version/size negotiation, with no implicit tail-field compatibility.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Version {
    /// ABI major, currently one.
    pub major: u32,
    /// ABI minor, currently zero.
    pub minor: u32,
    /// Size of this negotiation structure in bytes, currently sixteen.
    pub struct_size: u32,
    /// Must be zero.
    pub reserved: u32,
}
impl Default for Version {
    fn default() -> Self {
        Self {
            major: 1,
            minor: 0,
            struct_size: 16,
            reserved: 0,
        }
    }
}
impl Version {
    pub(crate) fn validate(self) -> Result<(), Status> {
        if self.major != 1 || self.minor != 0 {
            return Err(Status::VERSION);
        }
        if self.struct_size != 16 || self.reserved != 0 {
            return Err(Status::ARGUMENT);
        }
        Ok(())
    }
}

/// Stable numeric status, without a Rust enum crossing a prospective ABI.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status(pub u32);
impl Status {
    /// Successful operation.
    pub const OK: Self = Self(0);
    /// Unsupported exact version.
    pub const VERSION: Self = Self(1);
    /// Invalid size, flags, range or input.
    pub const ARGUMENT: Self = Self(2);
    /// Unknown, stale, wrong-kind or wrong-owner handle.
    pub const HANDLE: Self = Self(3);
    /// Bounded capacity or monotonic ID space exhausted.
    pub const CAPACITY: Self = Self(4);
    /// Fallible byte allocation failed.
    pub const ALLOCATION: Self = Self(5);
    /// Unwinding panic contained; this boundary is now poisoned.
    pub const PANIC: Self = Self(6);
    /// Prior panic permanently revoked this boundary.
    pub const POISONED: Self = Self(7);

    /// Static non-sensitive error description, never containing caller data.
    pub fn description(self) -> &'static str {
        match self {
            Self::OK => "ok",
            Self::VERSION => "unsupported ABI version",
            Self::ARGUMENT => "invalid argument",
            Self::HANDLE => "invalid handle",
            Self::CAPACITY => "capacity unavailable",
            Self::ALLOCATION => "allocation unavailable",
            Self::PANIC => "panic contained",
            Self::POISONED => "boundary unavailable",
            _ => "unknown status",
        }
    }
}

/// Candidate by-value result. Every error has a zero value and reserved field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reply {
    /// Numeric Status value.
    pub status: u32,
    /// Always zero.
    pub reserved: u32,
    /// Operation-specific value, zero on error.
    pub value: u64,
}
impl Reply {
    pub(crate) fn from_result(result: Result<u64, Status>) -> Self {
        match result {
            Ok(value) => Self {
                status: 0,
                reserved: 0,
                value,
            },
            Err(status) => Self {
                status: status.0,
                reserved: 0,
                value: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_and_offsets_are_fixed_width() {
        assert_eq!(std::mem::size_of::<Version>(), 16);
        assert_eq!(std::mem::offset_of!(Version, reserved), 12);
        assert_eq!(std::mem::size_of::<Reply>(), 16);
        assert_eq!(std::mem::offset_of!(Reply, value), 8);
        assert_eq!(std::mem::size_of::<Status>(), 4);
        for status in [
            Status::VERSION,
            Status::ARGUMENT,
            Status::HANDLE,
            Status::CAPACITY,
            Status::ALLOCATION,
            Status::PANIC,
            Status::POISONED,
        ] {
            assert_eq!(
                Reply::from_result(Err(status)),
                Reply {
                    status: status.0,
                    reserved: 0,
                    value: 0
                }
            );
            assert!(!status.description().is_empty());
        }
    }
}
