//! Independent close deadlines; unchanged lifecycle cancellation and join behavior.
use meari_native::infrastructure::window::NativeWindowWake;
use std::time::Duration;
pub(super) struct CloseDeadline {
    cancel: std::sync::mpsc::Sender<()>,
    worker: Option<std::thread::JoinHandle<()>>,
    cancel_watchdog: std::sync::mpsc::Sender<()>,
    watchdog: Option<std::thread::JoinHandle<()>>,
}
impl CloseDeadline {
    pub(super) fn start(wake: NativeWindowWake, seconds: u64) -> Self {
        Self::start_with(
            move || {
                let _ = wake.request_close();
            },
            Duration::from_secs(seconds),
            Duration::from_secs(seconds + 15),
            || {
                eprintln!("native: incomplete cleanup at independent hard deadline");
                std::process::exit(78);
            },
        )
    }
    fn start_with(
        close: impl FnOnce() + Send + 'static,
        close_after: Duration,
        hard_after: Duration,
        fail_stop: impl FnOnce() + Send + 'static,
    ) -> Self {
        use std::sync::mpsc::{RecvTimeoutError, channel};
        let (cancel, receiver) = channel();
        let (cancel_watchdog, watchdog_receiver) = channel();
        let watchdog = std::thread::spawn(move || {
            if matches!(
                watchdog_receiver.recv_timeout(hard_after),
                Err(RecvTimeoutError::Timeout)
            ) {
                fail_stop();
            }
        });
        let worker = std::thread::spawn(move || {
            if matches!(
                receiver.recv_timeout(close_after),
                Err(RecvTimeoutError::Timeout)
            ) {
                eprintln!("native: independent native close deadline");
                close();
            }
        });
        Self {
            cancel,
            worker: Some(worker),
            cancel_watchdog,
            watchdog: Some(watchdog),
        }
    }
}
impl Drop for CloseDeadline {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = self.cancel_watchdog.send(());
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
    }
}
#[cfg(test)]
mod deadline_tests {
    use super::*;
    #[test]
    fn hard_deadline_survives_a_blocked_close_wake() {
        let (release, blocked) = std::sync::mpsc::channel::<()>();
        let (fired, receipt) = std::sync::mpsc::channel();
        let deadline = CloseDeadline::start_with(
            move || {
                let _ = blocked.recv();
            },
            Duration::from_millis(5),
            Duration::from_millis(20),
            move || {
                fired.send(()).unwrap();
            },
        );
        receipt
            .recv_timeout(Duration::from_secs(2))
            .expect("blocked close suppressed hard deadline");
        release.send(()).unwrap();
        drop(deadline);
    }
    #[test]
    fn completed_lifecycle_cancels_and_joins_both_workers() {
        let (fired, receipt) = std::sync::mpsc::channel();
        let deadline = CloseDeadline::start_with(
            || panic!("completed lifecycle must not request close"),
            Duration::from_secs(60),
            Duration::from_secs(75),
            move || {
                let _ = fired.send(());
            },
        );
        drop(deadline);
        assert!(matches!(
            receipt.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        ));
    }
}
