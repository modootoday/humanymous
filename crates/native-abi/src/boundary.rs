//! Panic-to-status containment for the future export composition root.
use crate::{Registry, Reply, Status};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Single owner that permanently revokes further operations after a panic.
///
/// The host's panic hook remains installed. Abort, allocation abort and native
/// memory faults are not contained. This safe Rust type exports no C symbol.
#[derive(Default)]
pub struct Boundary {
    registry: Registry,
    poisoned: bool,
}
impl Boundary {
    /// Permanently revoke operations and drop every owned zeroizing buffer.
    ///
    /// This is cleanup only; there is no recovery/reset path after revocation.
    pub fn revoke(&mut self) {
        self.poisoned = true;
        self.registry.revoke_all();
    }
    /// Execute one call with exclusive ownership; all errors return zero value.
    ///
    /// A panic permanently poisons the boundary, so partially modified state is
    /// never reused. The caught payload is leaked because its destructor can
    /// itself panic. At most one payload is retained per boundary instance.
    pub fn call(&mut self, operation: impl FnOnce(&mut Registry) -> Result<u64, Status>) -> Reply {
        if self.poisoned {
            return Reply::from_result(Err(Status::POISONED));
        }
        match catch_unwind(AssertUnwindSafe(|| operation(&mut self.registry))) {
            Ok(result) => Reply::from_result(result),
            Err(payload) => {
                std::mem::forget(payload);
                self.revoke();
                Reply::from_result(Err(Status::PANIC))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panic_revokes_partially_modified_state_without_running_payload_destructor() {
        struct Explosive;
        impl Drop for Explosive {
            fn drop(&mut self) {
                panic!("payload destructor");
            }
        }
        let mut b = Boundary::default();
        let reply = b.call(|r| {
            let c = r.open(crate::Version::default())?;
            let h = r.allocate(c, 8)?;
            r.write(c, h, 0, b"secret!!")?;
            std::panic::panic_any(Explosive)
        });
        assert_eq!(b.registry.allocated_bytes(), 0);
        assert_eq!(
            reply,
            Reply {
                status: Status::PANIC.0,
                reserved: 0,
                value: 0
            }
        );
        let reply = b.call(|_| panic!("must never execute"));
        assert_eq!(
            reply,
            Reply {
                status: Status::POISONED.0,
                reserved: 0,
                value: 0
            }
        );
    }
    #[test]
    fn normal_errors_do_not_poison() {
        let mut b = Boundary::default();
        assert_eq!(b.call(|_| Err(Status::HANDLE)).value, 0);
        assert_eq!(b.call(|r| r.open(crate::Version::default())).status, 0);
    }
    #[test]
    fn explicit_revocation_cleans_buffers_and_cannot_be_reset() {
        let mut b = Boundary::default();
        assert_eq!(
            b.call(|r| {
                let c = r.open(crate::Version::default())?;
                r.allocate(c, 64)
            })
            .status,
            0
        );
        assert_eq!(b.registry.allocated_bytes(), 64);
        b.revoke();
        assert_eq!(b.registry.allocated_bytes(), 0);
        b.revoke();
        assert_eq!(
            b.call(|_| panic!("revoked operation ran")).status,
            Status::POISONED.0
        );
    }
}
