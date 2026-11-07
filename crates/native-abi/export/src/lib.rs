//! Narrow pointer-free C export over the single safe native ABI implementation.
//!
//! Only the unmangled export attribute is excepted. There are no unsafe blocks,
//! foreign pointers, callbacks, workers, host process exits or copied policies.
#![deny(unsafe_code)]

use humanymous_native_abi::{Boundary, Command, Reply, Response, Status};
use std::panic::catch_unwind;
use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};

struct ExportOwner {
    boundary: LazyLock<Mutex<Boundary>>,
    revoked: AtomicBool,
}
impl ExportOwner {
    const fn new() -> Self {
        Self {
            boundary: LazyLock::new(|| Mutex::new(Boundary::default())),
            revoked: AtomicBool::new(false),
        }
    }

    fn call(&self, command: Command) -> Response {
        if self.revoked.load(Ordering::Acquire) {
            return failure(Status::POISONED);
        }
        match catch_unwind(|| {
            let mut boundary = match self.boundary.lock() {
                Ok(boundary) => boundary,
                Err(poisoned) => {
                    self.revoked.store(true, Ordering::Release);
                    // Cleanup only: never clear poison or resume operations.
                    poisoned.into_inner().revoke();
                    return failure(Status::POISONED);
                }
            };
            if self.revoked.load(Ordering::Acquire) {
                return failure(Status::POISONED);
            }
            let response = boundary.dispatch(command);
            if response.reply.status == Status::PANIC.0
                || response.reply.status == Status::POISONED.0
            {
                self.revoked.store(true, Ordering::Release);
            }
            response
        }) {
            Ok(response) => response,
            Err(payload) => {
                self.revoked.store(true, Ordering::Release);
                std::mem::forget(payload);
                self.revoke_initialized();
                failure(Status::PANIC)
            }
        }
    }

    fn revoke_initialized(&self) {
        // get does not retry a failed initializer. The unwound call released
        // its guard before cleanup takes it; never clear a poison flag.
        if let Some(owner) = LazyLock::get(&self.boundary) {
            match owner.lock() {
                Ok(mut boundary) => boundary.revoke(),
                Err(poisoned) => poisoned.into_inner().revoke(),
            }
        }
    }
}

fn failure(status: Status) -> Response {
    Response {
        reply: Reply {
            status: status.0,
            reserved: 0,
            value: 0,
        },
        data: [0; 32],
    }
}

static OWNER: ExportOwner = ExportOwner::new();

/// Execute one versioned pointer-free native buffer/lifecycle command.
///
/// All calls serialize on one library-lifetime owner. No callbacks or workers
/// are started. No C++ exception may enter Rust. Reentry/signal-handler/post-fork
/// use is unsupported. Complete calls and close contexts before unload; never
/// retain handles across unload/reload. Panic hooks can log/abort before unwind
/// containment; allocation abort/native faults are not contained. The host must
/// ensure this uniquely prefixed symbol has one definition in the link image.
#[expect(
    unsafe_code,
    reason = "root-approved no_mangle attribute only; pointer-free by-value ABI"
)]
#[unsafe(no_mangle)]
pub extern "C" fn humanymous_native_abi_v1(command: Command) -> Response {
    OWNER.call(command)
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
    fn mutex_poison_fails_closed_without_reusing_state() {
        let owner = ExportOwner::new();
        let _ = catch_unwind(|| {
            let mut boundary = owner.boundary.lock().unwrap();
            assert_eq!(
                boundary.dispatch(command(Command::OPEN, 0, 0)).reply.status,
                0
            );
            panic!("synthetic owner-lock panic");
        });
        for _ in 0..2 {
            let result = owner.call(command(Command::OPEN, 0, 0));
            assert_eq!(result.reply.status, Status::POISONED.0);
            assert_eq!(result.reply.value, 0);
            assert_eq!(result.data, [0; 32]);
        }
    }
    #[test]
    fn lazy_initialization_panic_is_contained_and_permanently_revoked() {
        let owner = ExportOwner {
            boundary: LazyLock::new(|| panic!("synthetic initialization panic")),
            revoked: AtomicBool::new(false),
        };
        assert_eq!(
            owner.call(command(Command::OPEN, 0, 0)).reply.status,
            Status::PANIC.0
        );
        assert_eq!(
            owner.call(command(Command::OPEN, 0, 0)).reply.status,
            Status::POISONED.0
        );
    }
    #[test]
    fn outer_cleanup_does_not_initialize_or_resume_a_boundary() {
        let owner = ExportOwner::new();
        owner.revoke_initialized();
        assert!(LazyLock::get(&owner.boundary).is_none());
        let c = owner.call(command(Command::OPEN, 0, 0)).reply.value;
        assert_eq!(
            owner
                .call(Command {
                    length: 8,
                    ..command(Command::ALLOCATE, c, 0)
                })
                .reply
                .status,
            0
        );
        owner.revoked.store(true, Ordering::Release);
        owner.revoke_initialized();
        assert_eq!(
            owner
                .boundary
                .lock()
                .unwrap()
                .dispatch(command(Command::OPEN, 0, 0))
                .reply
                .status,
            Status::POISONED.0
        );
    }
    #[test]
    fn concurrent_callers_share_one_unique_handle_owner() {
        let owner = ExportOwner::new();
        let handles = std::thread::scope(|scope| {
            let calls: Vec<_> = (0..16)
                .map(|_| {
                    scope.spawn(|| {
                        let opened = owner.call(command(Command::OPEN, 0, 0));
                        assert_eq!(opened.reply.status, 0);
                        let context = opened.reply.value;
                        let allocation = owner.call(Command {
                            length: 32,
                            ..command(Command::ALLOCATE, context, 0)
                        });
                        assert_eq!(allocation.reply.status, 0);
                        let buffer = allocation.reply.value;
                        let mut write = Command {
                            length: 32,
                            ..command(Command::WRITE, context, buffer)
                        };
                        write.data.fill(context as u8);
                        assert_eq!(owner.call(write).reply.status, 0);
                        assert_eq!(
                            owner
                                .call(Command {
                                    length: 32,
                                    ..command(Command::READ, context, buffer)
                                })
                                .data,
                            [context as u8; 32]
                        );
                        assert_eq!(
                            owner.call(command(Command::CLOSE, context, 0)).reply.status,
                            0
                        );
                        assert_eq!(
                            owner
                                .call(command(Command::LENGTH, context, buffer))
                                .reply
                                .status,
                            Status::HANDLE.0
                        );
                        [context, buffer]
                    })
                })
                .collect();
            calls
                .into_iter()
                .flat_map(|c| c.join().unwrap())
                .collect::<std::collections::BTreeSet<_>>()
        });
        assert_eq!(handles.len(), 32);
    }
    #[test]
    fn close_and_access_linearize_under_one_mutex() {
        let owner = ExportOwner::new();
        let c = owner.call(command(Command::OPEN, 0, 0)).reply.value;
        let h = owner
            .call(Command {
                length: 1,
                ..command(Command::ALLOCATE, c, 0)
            })
            .reply
            .value;
        std::thread::scope(|scope| {
            let read = scope.spawn(|| {
                owner.call(Command {
                    length: 1,
                    ..command(Command::READ, c, h)
                })
            });
            let close = scope.spawn(|| owner.call(command(Command::CLOSE, c, 0)));
            let status = read.join().unwrap().reply.status;
            assert!(status == Status::OK.0 || status == Status::HANDLE.0);
            assert_eq!(close.join().unwrap().reply.status, 0);
        });
        assert_eq!(
            owner.call(command(Command::LENGTH, c, h)).reply.status,
            Status::HANDLE.0
        );
    }
    #[test]
    fn real_export_function_consumes_the_same_safe_dispatch() {
        let opened = humanymous_native_abi_v1(command(Command::OPEN, 0, 0));
        assert_eq!(opened.reply.status, 0);
        assert_eq!(
            humanymous_native_abi_v1(command(Command::CLOSE, opened.reply.value, 0))
                .reply
                .status,
            0
        );
    }
}
