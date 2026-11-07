//! Pure bounded transport/state tests. No process, thread or native qualification.
use super::*;
use std::{
    io::{Cursor, Read, Write},
    time::Duration,
};

fn bytes(n: usize) -> Box<[u8]> {
    vec![b'x'; n].into_boxed_slice()
}
type InputReceivers = (Receiver<Box<[u8]>>, Receiver<Box<[u8]>>);
fn mailbox() -> (Input, InputReceivers) {
    let mut input = Input::new();
    let (send, startup) = mpsc::sync_channel(1);
    input.startup = Some(send);
    let (send, control) = mpsc::sync_channel(CONTROL_SLOTS);
    input.control = Some(send);
    (input, (startup, control))
}
#[test]
fn stream_limits_are_positive_finite_and_closed() {
    assert!(InteractiveLimits::new(1, 262144).is_ok());
    for (a, b) in [(0, 1), (1, 0), (262145, 1), (1, usize::MAX)] {
        assert!(InteractiveLimits::new(a, b).is_err());
    }
}
#[test]
fn startup_is_once_and_control_before_startup_fails_sticky() {
    let (mut input, _) = mailbox();
    assert!(matches!(
        input.offer(bytes(1), false, true),
        EnqueueResult::Failed(_)
    ));
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Failed(_)
    ));
    let (mut input, (_startup, _control)) = mailbox();
    assert!(matches!(
        input.offer(bytes(STARTUP_MAX), true, true),
        EnqueueResult::Accepted
    ));
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Failed(_)
    ));
}
#[test]
fn oversized_expired_and_empty_offers_transfer_no_ownership() {
    for (n, timely) in [(STARTUP_MAX + 1, true), (1, false), (0, true)] {
        let (mut input, (_startup, _control)) = mailbox();
        let payload = bytes(n);
        let pointer = payload.as_ptr();
        match input.offer(payload, true, timely) {
            EnqueueResult::Failed(same) => assert_eq!(same.as_ptr(), pointer),
            _ => panic!("offer accepted"),
        }
        assert_eq!(input.accepted, 0);
    }
}
#[test]
fn full_control_queue_returns_same_box_and_remains_failed() {
    let (mut input, (_startup, control)) = mailbox();
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Accepted
    ));
    for _ in 0..CONTROL_SLOTS {
        assert!(matches!(
            input.offer(bytes(CONTROL_MAX), false, true),
            EnqueueResult::Accepted
        ));
    }
    let payload = bytes(1);
    let pointer = payload.as_ptr();
    match input.offer(payload, false, true) {
        EnqueueResult::Full(same) => assert_eq!(same.as_ptr(), pointer),
        _ => panic!("full queue accepted"),
    }
    let _ = control.try_recv().unwrap();
    assert!(matches!(
        input.offer(bytes(1), false, true),
        EnqueueResult::Failed(_)
    ));
    assert_eq!(input.accepted, 65);
}
#[test]
fn close_needs_no_free_slot_and_preserves_accepted_fifo() {
    let (mut input, (startup, control)) = mailbox();
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Accepted
    ));
    for i in 0..CONTROL_SLOTS {
        assert!(matches!(
            input.offer(vec![i as u8].into_boxed_slice(), false, true),
            EnqueueResult::Accepted
        ));
    }
    input.close();
    assert!(matches!(
        input.offer(bytes(1), false, true),
        EnqueueResult::Closed(_)
    ));
    let written = Arc::new(AtomicUsize::new(0));
    let mut destination = Vec::new();
    transport::input(
        &mut destination,
        startup,
        control,
        input.cancel.clone(),
        written.clone(),
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(destination[0], b'x');
    assert_eq!(&destination[1..], &(0..64u8).collect::<Vec<_>>());
    assert_eq!(written.load(Ordering::Acquire), 65);
}
#[test]
fn disconnected_writer_and_oversized_control_fail_sticky() {
    let (mut input, (startup, control)) = mailbox();
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Accepted
    ));
    assert!(matches!(
        input.offer(bytes(CONTROL_MAX + 1), false, true),
        EnqueueResult::Failed(_)
    ));
    drop(startup);
    drop(control);
    let (mut input, receivers) = mailbox();
    drop(receivers);
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Failed(_)
    ));
}
#[test]
fn cancelled_or_expired_writer_does_not_deliver_queued_startup() {
    for cancelled in [true, false] {
        let (mut input, (startup, control)) = mailbox();
        assert!(matches!(
            input.offer(bytes(2), true, true),
            EnqueueResult::Accepted
        ));
        input.close();
        input.cancel.store(cancelled, Ordering::Release);
        let until = if cancelled {
            Instant::now() + Duration::from_secs(1)
        } else {
            Instant::now()
        };
        let mut destination = Vec::new();
        assert!(
            transport::input(
                &mut destination,
                startup,
                control,
                input.cancel,
                input.written.clone(),
                until
            )
            .is_err()
        );
        assert!(destination.is_empty());
        assert_eq!(input.written.load(Ordering::Acquire), 0);
    }
}
#[test]
fn output_full_is_error_independent_of_retained_chunks() {
    let (send, receive) = mpsc::sync_channel(OUTPUT_SLOTS);
    let mut source = Cursor::new(vec![b'z'; CHUNK * (OUTPUT_SLOTS + 1)]);
    assert!(transport::read_output(&mut source, 262144, send).is_err());
    for _ in 0..OUTPUT_SLOTS {
        assert_eq!(receive.try_recv().unwrap().bytes(), vec![b'z'; CHUNK]);
    }
    assert!(receive.try_recv().is_err());
}
#[test]
fn lifetime_cap_accepts_exact_eof_and_rejects_one_more_byte() {
    for extra in [0, 1] {
        let (send, receive) = mpsc::sync_channel(OUTPUT_SLOTS);
        let result =
            transport::read_output(&mut Cursor::new(vec![b'a'; CHUNK + extra]), CHUNK, send);
        assert_eq!(result.is_ok(), extra == 0);
        assert_eq!(receive.try_recv().unwrap().bytes().len(), CHUNK);
        assert!(receive.try_recv().is_err());
    }
}
struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic read failure"))
    }
}
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic write failure"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn read_and_write_errors_never_become_eof_or_delivery() {
    let (send, _receive) = mpsc::sync_channel(OUTPUT_SLOTS);
    assert!(transport::read_output(&mut Broken, 10, send).is_err());
    let (mut input, (startup, control)) = mailbox();
    assert!(matches!(
        input.offer(bytes(1), true, true),
        EnqueueResult::Accepted
    ));
    input.close();
    assert!(
        transport::input(
            Broken,
            startup,
            control,
            input.cancel,
            input.written.clone(),
            Instant::now() + Duration::from_secs(1)
        )
        .is_err()
    );
    assert_eq!(input.written.load(Ordering::Acquire), 0);
}
