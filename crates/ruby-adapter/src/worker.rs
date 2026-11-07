//! Request-owned Rust worker; cancellation and joining require no Ruby calls.
pub(crate) struct DecisionWorker {
    cancel: Option<tokio::sync::watch::Sender<bool>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl DecisionWorker {
    pub(crate) fn new(
        cancel: tokio::sync::watch::Sender<bool>,
        thread: std::thread::JoinHandle<()>,
    ) -> Self {
        Self {
            cancel: Some(cancel),
            thread: Some(thread),
        }
    }
}

impl Drop for DecisionWorker {
    fn drop(&mut self) {
        // Closing the sender wakes the receiver even when its value was false.
        self.cancel.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn drop_closes_cancellation_and_joins_before_returning() {
        let (cancel, mut receiver) = tokio::sync::watch::channel(false);
        let completed = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&completed);
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            assert!(runtime.block_on(receiver.changed()).is_err());
            observed.store(true, Ordering::Release);
        });
        drop(DecisionWorker::new(cancel, thread));
        assert!(
            completed.load(Ordering::Acquire),
            "drop must reap the owned worker, not detach it"
        );
    }

    #[test]
    fn worker_panic_does_not_cause_a_second_drop_panic() {
        let (cancel, _receiver) = tokio::sync::watch::channel(false);
        let completed = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&completed);
        let thread = std::thread::spawn(move || {
            observed.store(true, Ordering::Release);
            panic!("owned worker fixture panic");
        });
        drop(DecisionWorker::new(cancel, thread));
        assert!(completed.load(Ordering::Acquire));
    }
}
