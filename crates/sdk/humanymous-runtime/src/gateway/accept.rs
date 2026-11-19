//! Accept with Go `net/http` `Server.Serve` retry semantics: a transient
//! accept error (descriptor or buffer exhaustion, an aborted or reset
//! pending connection, an interrupted call) is retried with exponential
//! backoff from 5 ms to 1 s instead of stopping the listener. Any other
//! accept error is returned, and the service then stops every listener
//! (fail closed).
use std::{future::Future, io, net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};

const FIRST_BACKOFF: Duration = Duration::from_millis(5);
const MAX_BACKOFF: Duration = Duration::from_secs(1);

/// Whether an accept error is temporary (Go `net.Error` Temporary class).
pub(crate) fn transient(error: &io::Error) -> bool {
    if matches!(
        error.kind(),
        io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::Interrupted
            | io::ErrorKind::WouldBlock
            | io::ErrorKind::OutOfMemory
    ) {
        return true;
    }
    error
        .raw_os_error()
        .is_some_and(|code| exhausted_codes().contains(&code))
}

/// Descriptor, buffer and memory exhaustion and protocol-error codes that
/// Linux accept(2) documents as retryable.
#[cfg(unix)]
fn exhausted_codes() -> [i32; 5] {
    use rustix::io::Errno;
    [
        Errno::MFILE.raw_os_error(),
        Errno::NFILE.raw_os_error(),
        Errno::NOBUFS.raw_os_error(),
        Errno::NOMEM.raw_os_error(),
        Errno::PROTO.raw_os_error(),
    ]
}

/// WSAEMFILE, WSAENOBUFS, WSAECONNABORTED, WSAECONNRESET and
/// WSAEINTR.
#[cfg(windows)]
fn exhausted_codes() -> [i32; 5] {
    [10024, 10055, 10053, 10054, 10004]
}

#[cfg(not(any(unix, windows)))]
fn exhausted_codes() -> [i32; 0] {
    []
}

/// Run `attempt` until it succeeds or fails with a non-transient error,
/// sleeping with exponential backoff between transient failures.
pub(crate) async fn retrying<T, F, Fut>(mut attempt: F) -> io::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = io::Result<T>>,
{
    let mut backoff = FIRST_BACKOFF;
    loop {
        match attempt().await {
            Err(error) if transient(&error) => {
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            result => return result,
        }
    }
}

/// [`TcpListener::accept`] that survives transient accept errors.
pub(crate) async fn accept(listener: &TcpListener) -> io::Result<(TcpStream, SocketAddr)> {
    retrying(|| listener.accept()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustion_and_aborted_connections_are_transient() {
        for code in exhausted_codes() {
            assert!(transient(&io::Error::from_raw_os_error(code)), "{code}");
        }
        assert!(transient(&io::ErrorKind::ConnectionAborted.into()));
        assert!(transient(&io::ErrorKind::Interrupted.into()));
        assert!(!transient(&io::ErrorKind::InvalidInput.into()));
        assert!(!transient(&io::ErrorKind::PermissionDenied.into()));
        assert!(!transient(&io::Error::other("closed")));
    }

    #[tokio::test]
    async fn transient_failures_are_retried_and_others_returned() {
        let exhausted = exhausted_codes()[0];
        let mut script = vec![
            Err(io::Error::from_raw_os_error(exhausted)),
            Err(io::Error::from_raw_os_error(exhausted)),
            Ok(7),
        ]
        .into_iter();
        let started = std::time::Instant::now();
        let value = retrying(|| std::future::ready(script.next().unwrap()))
            .await
            .unwrap();
        assert_eq!(value, 7);
        // Two backoffs, 5 ms and 10 ms, were slept.
        assert!(started.elapsed() >= Duration::from_millis(15));
        let mut script = vec![Err(io::ErrorKind::InvalidInput.into()), Ok(1)].into_iter();
        let error = retrying(|| std::future::ready(script.next().unwrap()))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    /// A real listener still accepts through the retrying wrapper.
    #[tokio::test]
    async fn accept_serves_a_real_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = tokio::spawn(TcpStream::connect(address));
        let (_stream, peer) = accept(&listener).await.unwrap();
        assert!(peer.ip().is_loopback());
        client.await.unwrap().unwrap();
    }
}
