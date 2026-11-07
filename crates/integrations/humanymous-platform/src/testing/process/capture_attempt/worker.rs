use super::*;

pub(in crate::testing::process) struct Worker<T> {
    handle: Option<JoinHandle<()>>,
    receiver: Receiver<io::Result<T>>,
    result: Option<io::Result<T>>,
    joined: bool,
}
impl<T: Send + 'static> Worker<T> {
    pub(in crate::testing::process) fn start(
        name: &str,
        operation: impl FnOnce() -> io::Result<T> + Send + 'static,
    ) -> io::Result<Self> {
        let (send, receiver) = mpsc::sync_channel(1);
        let handle = thread::Builder::new().name(name.into()).spawn(move || {
            let _ = send.send(operation());
        })?;
        Ok(Self {
            handle: Some(handle),
            receiver,
            result: None,
            joined: false,
        })
    }
    pub(in crate::testing::process) fn poll(&mut self) {
        if self.result.is_none() {
            self.result = match self.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err(io::Error::other("capture worker outcome unavailable")))
                }
            };
        }
    }
    pub(in crate::testing::process) fn failed(&self) -> bool {
        self.result.as_ref().is_some_and(Result::is_err)
    }
    pub(in crate::testing::process) fn succeeded(&self) -> bool {
        self.result.as_ref().is_some_and(Result::is_ok)
    }
    pub(in crate::testing::process) fn joined(&self) -> bool {
        self.joined
    }
    pub(in crate::testing::process) fn join_ready(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }
    // Potentially blocking final operation, NEVER called by poll.
    pub(in crate::testing::process) fn join(&mut self) -> bool {
        let panicked = self
            .handle
            .take()
            .is_some_and(|handle| handle.join().is_err());
        self.joined = true;
        self.poll();
        panicked
    }
    pub(in crate::testing::process) fn into_output(self) -> Option<T> {
        if self.joined {
            self.result.and_then(Result::ok)
        } else {
            None
        }
    }
}
