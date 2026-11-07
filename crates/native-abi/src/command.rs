//! Pointer-free fixed-width command surface, callable from safe Rust only.
use crate::{Boundary, Registry, Reply, Status, Version};

/// A bounded command candidate for one future by-value C entrypoint.
///
/// Reserved flags and unused fields must be zero. No host address, Rust layout,
/// callback or allocator crosses this surface. Each read/write is at most32bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Command {
    /// Exact version/size negotiation on every call.
    pub version: Version,
    /// One of this type's numeric operation constants.
    pub operation: u32,
    /// Must be zero.
    pub flags: u32,
    /// Owning live context, except OPEN requires zero.
    pub context: u64,
    /// Owned buffer, or zero for OPEN/CLOSE/ALLOCATE.
    pub buffer: u64,
    /// Byte offset for READ/WRITE; zero otherwise.
    pub offset: u64,
    /// Allocation length, or read/write chunk length; zero otherwise.
    pub length: u64,
    /// WRITE bytes, zero padding required; all zero for other operations.
    pub data: [u8; 32],
}
impl Command {
    /// Create one context; all payload fields must be zero.
    pub const OPEN: u32 = 1;
    /// Revoke one context and release all owned allocations.
    pub const CLOSE: u32 = 2;
    /// Allocate one zero-filled owned buffer.
    pub const ALLOCATE: u32 = 3;
    /// Zeroize and release one owned buffer.
    pub const FREE: u32 = 4;
    /// Return initialized buffer length.
    pub const LENGTH: u32 = 5;
    /// Copy at most32inline bytes into one buffer.
    pub const WRITE: u32 = 6;
    /// Return at most32inline bytes from one buffer.
    pub const READ: u32 = 7;

    fn execute(self, registry: &mut Registry, output: &mut [u8; 32]) -> Result<u64, Status> {
        self.version.validate()?;
        if self.flags != 0
            || (self.operation != Self::WRITE && self.data != [0; 32])
            || (self.operation != Self::READ && self.operation != Self::WRITE && self.offset != 0)
        {
            return Err(Status::ARGUMENT);
        }
        match self.operation {
            Self::OPEN => {
                if self.context != 0 || self.buffer != 0 || self.length != 0 {
                    return Err(Status::ARGUMENT);
                }
                registry.open(self.version)
            }
            Self::CLOSE => {
                if self.buffer != 0 || self.length != 0 {
                    return Err(Status::ARGUMENT);
                }
                registry.close(self.context)?;
                Ok(0)
            }
            Self::ALLOCATE => {
                if self.buffer != 0 {
                    return Err(Status::ARGUMENT);
                }
                registry.allocate(self.context, self.length)
            }
            Self::FREE | Self::LENGTH => {
                if self.length != 0 {
                    return Err(Status::ARGUMENT);
                }
                if self.operation == Self::FREE {
                    registry.free(self.context, self.buffer)?;
                    Ok(0)
                } else {
                    registry.length(self.context, self.buffer)
                }
            }
            Self::READ | Self::WRITE => {
                if self.length > 32 {
                    return Err(Status::ARGUMENT);
                }
                let length = self.length as usize;
                if self.operation == Self::WRITE {
                    if self.data[length..].iter().any(|&b| b != 0) {
                        return Err(Status::ARGUMENT);
                    }
                    registry.write(self.context, self.buffer, self.offset, &self.data[..length])?;
                } else {
                    registry.read(
                        self.context,
                        self.buffer,
                        self.offset,
                        &mut output[..length],
                    )?;
                }
                Ok(self.length)
            }
            _ => Err(Status::ARGUMENT),
        }
    }
}

/// Fixed-size initialized reply. Data is all zero on every error/non-read call.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Response {
    /// Stable status, zero reserved field, and operation-specific scalar.
    pub reply: Reply,
    /// READ result only, with unused tail initialized zero.
    pub data: [u8; 32],
}
impl Boundary {
    /// Execute a pointer-free command using the same Registry and panic owner.
    ///
    /// This is a safe Rust method, not an exported foreign-language entrypoint.
    pub fn dispatch(&mut self, command: Command) -> Response {
        let mut data = [0; 32];
        let reply = self.call(|registry| command.execute(registry, &mut data));
        if reply.status != Status::OK.0 {
            data.fill(0);
        }
        Response { reply, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(operation: u32, context: u64, buffer: u64) -> Command {
        Command {
            operation,
            context,
            buffer,
            ..Command::default()
        }
    }
    #[test]
    fn pointer_free_command_sequence_preserves_ownership_and_initialization() {
        let mut boundary = Boundary::default();
        let opened = boundary.dispatch(command(Command::OPEN, 0, 0));
        assert_eq!(opened.reply.status, 0);
        let c = opened.reply.value;
        let allocated = boundary.dispatch(Command {
            length: 64,
            ..command(Command::ALLOCATE, c, 0)
        });
        assert_eq!(allocated.reply.status, 0);
        let h = allocated.reply.value;
        let mut write = Command {
            length: 3,
            offset: 31,
            ..command(Command::WRITE, c, h)
        };
        write.data[..3].copy_from_slice(&[0, 255, 42]);
        assert_eq!(boundary.dispatch(write).reply.value, 3);
        let read = boundary.dispatch(Command {
            length: 5,
            offset: 30,
            ..command(Command::READ, c, h)
        });
        assert_eq!(read.reply.status, 0);
        assert_eq!(&read.data[..5], &[0, 0, 255, 42, 0]);
        assert_eq!(&read.data[5..], &[0; 27]);
        assert_eq!(
            boundary
                .dispatch(command(Command::LENGTH, c, h))
                .reply
                .value,
            64
        );
        assert_eq!(
            boundary.dispatch(command(Command::FREE, c, h)).reply.status,
            0
        );
        assert_eq!(
            boundary.dispatch(command(Command::READ, c, h)).reply.status,
            Status::HANDLE.0
        );
        assert_eq!(
            boundary
                .dispatch(command(Command::CLOSE, c, 0))
                .reply
                .status,
            0
        );
    }
    #[test]
    fn invalid_commands_zero_outputs_and_leave_live_data_unchanged() {
        let mut b = Boundary::default();
        let c = b.dispatch(command(Command::OPEN, 0, 0)).reply.value;
        let h = b
            .dispatch(Command {
                length: 4,
                ..command(Command::ALLOCATE, c, 0)
            })
            .reply
            .value;
        let read = Command {
            length: 4,
            ..command(Command::READ, c, h)
        };
        let mut hidden = read;
        hidden.data[31] = 1;
        let mut bad_write = Command {
            length: 3,
            ..command(Command::WRITE, c, h)
        };
        bad_write.data[3] = 1;
        for cmd in [
            Command {
                operation: 0,
                ..Command::default()
            },
            Command { flags: 1, ..read },
            Command { length: 33, ..read },
            Command {
                offset: u64::MAX,
                ..read
            },
            Command {
                length: 1,
                ..command(Command::FREE, c, h)
            },
            Command {
                offset: 1,
                ..command(Command::LENGTH, c, h)
            },
            Command {
                buffer: h,
                ..command(Command::CLOSE, c, 0)
            },
            hidden,
            bad_write,
        ] {
            let reply = b.dispatch(cmd);
            assert_eq!(reply.reply.status, Status::ARGUMENT.0);
            assert_eq!(reply.reply.value, 0);
            assert_eq!(reply.data, [0; 32]);
        }
        assert_eq!(b.dispatch(read).data, [0; 32]);
        let incompatible = Command {
            version: Version {
                major: 2,
                ..Version::default()
            },
            ..read
        };
        assert_eq!(b.dispatch(incompatible).reply.status, Status::VERSION.0);
    }
    #[test]
    fn command_layout_has_no_pointer_width_dependency() {
        assert_eq!(std::mem::size_of::<Command>(), 88);
        assert_eq!(std::mem::offset_of!(Command, context), 24);
        assert_eq!(std::mem::offset_of!(Command, data), 56);
        assert_eq!(std::mem::size_of::<Response>(), 48);
        assert_eq!(std::mem::offset_of!(Response, data), 16);
    }
}
