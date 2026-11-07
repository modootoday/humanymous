//! Narrow pointer-free .NET export; all request/security ownership stays Rust.
#![deny(unsafe_code)]
use humanymous_dotnet_adapter::bridge::{Bridge, DECIDE};
use humanymous_native_abi::{Command, Reply, Response, Status};
use std::{
    panic::catch_unwind,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

static BRIDGE: LazyLock<Mutex<Bridge>> = LazyLock::new(|| Mutex::new(Bridge::default()));
static REVOKED: AtomicBool = AtomicBool::new(false);
static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("native adapter runtime unavailable")
});
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
fn revoke() {
    REVOKED.store(true, Ordering::Release);
    if let Some(bridge) = LazyLock::get(&BRIDGE) {
        match bridge.lock() {
            Ok(mut b) => b.revoke(),
            Err(p) => p.into_inner().revoke(),
        }
    }
}
fn invoke(c: Command) -> Response {
    if REVOKED.load(Ordering::Acquire) {
        return failure(Status::POISONED);
    }
    let result = catch_unwind(|| {
        if c.operation == DECIDE {
            let ticket = {
                let mut bridge = match BRIDGE.lock() {
                    Ok(b) => b,
                    Err(_) => return failure(Status::POISONED),
                };
                if REVOKED.load(Ordering::Acquire) {
                    return failure(Status::POISONED);
                }
                match bridge.prepare(c) {
                    Ok(t) => t,
                    Err(s) => return failure(s),
                }
            };
            let completion = RUNTIME.block_on(ticket.run());
            let mut bridge = match BRIDGE.lock() {
                Ok(b) => b,
                Err(_) => return failure(Status::POISONED),
            };
            if REVOKED.load(Ordering::Acquire) {
                return failure(Status::POISONED);
            }
            bridge.complete(completion)
        } else {
            let mut bridge = match BRIDGE.lock() {
                Ok(b) => b,
                Err(_) => return failure(Status::POISONED),
            };
            if REVOKED.load(Ordering::Acquire) {
                return failure(Status::POISONED);
            }
            bridge.dispatch(c)
        }
    });
    match result {
        Ok(response) => {
            if response.reply.status == Status::PANIC.0
                || response.reply.status == Status::POISONED.0
            {
                revoke();
            }
            response
        }
        Err(payload) => {
            std::mem::forget(payload);
            revoke();
            failure(Status::PANIC)
        }
    }
}

/// ABI1 by-value command; no foreign addresses, callbacks or allocators.
#[expect(
    unsafe_code,
    reason = "approved unmangled pointer-free export attribute only"
)]
#[unsafe(no_mangle)]
pub extern "C" fn humanymous_dotnet_v1(command: Command) -> Response {
    invoke(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_layout_and_invalid_operation_have_zero_payload() {
        assert_eq!(std::mem::size_of::<Command>(), 88);
        assert_eq!(std::mem::size_of::<Response>(), 48);
        let r = invoke(Command {
            operation: 999,
            ..Command::default()
        });
        assert_ne!(r.reply.status, 0);
        assert_eq!(r.reply.value, 0);
        assert_eq!(r.data, [0; 32]);
    }
}
