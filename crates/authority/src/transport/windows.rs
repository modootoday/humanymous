use crate::{
    configuration::{Configuration, Peer},
    decision::NativePeer,
};
use interprocess::os::windows::{
    named_pipe::{
        PipeListenerOptions, pipe_mode,
        tokio::{DuplexPipeStream, PipeListener},
    },
    security_descriptor::SecurityDescriptor,
};
use std::io;
pub(super) struct Listener {
    listener: PipeListener<pipe_mode::Bytes, pipe_mode::Bytes>,
}
impl Listener {
    pub fn bind(config: &Configuration) -> io::Result<Self> {
        let Peer::Windows { sid, .. } = &config.peer else {
            return Err(io::Error::other("Windows principal required"));
        };
        let name = config
            .endpoint
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
            .path(config.endpoint.as_str())
            .accept_remote(false)
            .inheritable(false)
            .security_descriptor(Some(sd))
            .create_tokio_duplex::<pipe_mode::Bytes>()?;
        Ok(Self { listener })
    }
    pub async fn accept(
        &self,
    ) -> io::Result<(DuplexPipeStream<pipe_mode::Bytes>, Option<NativePeer>)> {
        let stream = self.listener.accept().await?;
        let peer = stream.client_process_id().ok().map(NativePeer::Windows);
        Ok((stream, peer))
    }
}
