use crate::{configuration::Peer, decision::NativePeer};
use interprocess::os::windows::{
    named_pipe::{
        PipeListenerOptions, pipe_mode,
        tokio::{DuplexPipeStream, PipeListener},
    },
    security_descriptor::SecurityDescriptor,
};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
pub(super) struct Listener {
    listener: PipeListener<pipe_mode::Bytes, pipe_mode::Bytes>,
}
impl Listener {
    pub fn bind(endpoint: &str, peer: &Peer) -> io::Result<Self> {
        let Peer::Windows { sid, .. } = peer else {
            return Err(io::Error::other("Windows principal required"));
        };
        let name = endpoint
            .strip_prefix(r"\\.\pipe\")
            .ok_or_else(|| io::Error::other("local Windows pipe required"))?;
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || !crate::configuration::valid_sid(sid)
        {
            return Err(io::Error::other("invalid local pipe name/principal"));
        }
        // Protected DACL, no inherited/default grants. The configured SID is
        // validated before SDDL composition. First-instance protection and
        // PIPE_REJECT_REMOTE_CLIENTS are enforced by the library's creation API.
        let sddl = widestring::U16CString::from_str(format!("O:{sid}G:{sid}D:P(A;;GA;;;{sid})"))
            .map_err(io::Error::other)?;
        let sd = SecurityDescriptor::deserialize(&sddl)?;
        let listener = PipeListenerOptions::new()
            .path(endpoint)
            .accept_remote(false)
            .inheritable(false)
            .security_descriptor(Some(sd))
            .create_tokio_duplex::<pipe_mode::Bytes>()?;
        Ok(Self { listener })
    }
    pub async fn accept(&self) -> io::Result<(Stream, Option<NativePeer>)> {
        let stream = self.listener.accept().await?;
        let peer = stream.client_process_id().ok().map(NativePeer::Windows);
        Ok((Stream { inner: stream }, peer))
    }
}

// Never start blocking FlushFileBuffers or the dependency's deferred-close
// limbo pool. Kernel enqueue is asynchronous; the connection owner waits for
// peer close within its deadline. Cancellation intentionally discards leftovers.
pub(crate) struct Stream {
    inner: DuplexPipeStream<pipe_mode::Bytes>,
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.inner.assume_flushed();
    }
}
impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}
impl AsyncWrite for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
pub(crate) async fn connect(endpoint: &str, server: &Peer) -> io::Result<Stream> {
    use std::os::windows::{fs::OpenOptionsExt, io::OwnedHandle};
    let Peer::Windows { sid, pid } = server else {
        return Err(io::Error::other("Windows server identity required"));
    };
    let name = endpoint
        .strip_prefix(r"\\.\pipe\")
        .ok_or_else(|| io::Error::other("local pipe required"))?;
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        || *pid == 0
        || !crate::configuration::valid_sid(sid)
    {
        return Err(io::Error::other("invalid local pipe identity"));
    }
    // Immediate CreateFile, no retry/wait worker. Identification SQOS prevents
    // the server from using this client's token for privileged operations.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(0x40000000 | 0x00100000 | 0x00010000)
        .open(endpoint)?;
    let handle: OwnedHandle = file.into();
    let inner = DuplexPipeStream::<pipe_mode::Bytes>::try_from(handle)
        .map_err(|_| io::Error::other("pipe handle conversion failed"))?;
    if inner.is_server() || inner.server_process_id()? != *pid {
        return Err(io::Error::other("pipe server process mismatch"));
    }
    humanymous_native_security::storage::verify_windows_pipe_privacy(&inner, sid)?;
    Ok(Stream { inner })
}
