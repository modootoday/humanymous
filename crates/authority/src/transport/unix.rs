use crate::{configuration::Peer, decision::NativePeer};
use std::{
    io,
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    path::PathBuf,
};
pub(super) struct Listener {
    listener: tokio::net::UnixListener,
    path: PathBuf,
    inode: u64,
    device: u64,
}
impl Listener {
    pub fn bind(endpoint: &str, peer: &Peer) -> io::Result<Self> {
        let path = PathBuf::from(endpoint);
        let Peer::Unix { uid } = *peer else {
            return Err(io::Error::other("Unix principal required"));
        };
        if !path.is_absolute()
            || endpoint.as_bytes().contains(&0)
            || endpoint.len() > 100
            || path.file_name().is_none()
            || rustix::process::geteuid().as_raw() != uid
        {
            return Err(io::Error::other("invalid filesystem socket or owner"));
        }
        super::private_directory(
            path.parent()
                .ok_or_else(|| io::Error::other("socket parent"))?,
            peer,
        )?;
        // Never unlink an existing live/stale path on startup. The private 0700
        // parent closes the interval before socket chmod on macOS.
        let listener = tokio::net::UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        let m = std::fs::symlink_metadata(&path)?;
        if !m.file_type().is_socket() || m.uid() != uid || m.mode() & 0o777 != 0o600 {
            return Err(io::Error::other("socket ownership verification failed"));
        }
        Ok(Self {
            listener,
            path,
            inode: m.ino(),
            device: m.dev(),
        })
    }
    pub async fn accept(&self) -> io::Result<(tokio::net::UnixStream, Option<NativePeer>)> {
        let (stream, _) = self.listener.accept().await?;
        let peer = stream
            .peer_cred()
            .ok()
            .map(|peer| NativePeer::Unix(peer.uid()));
        Ok((stream, peer))
    }
}

pub(crate) async fn connect(endpoint: &str, server: &Peer) -> io::Result<tokio::net::UnixStream> {
    let Peer::Unix { uid } = server else {
        return Err(io::Error::other("Unix server identity required"));
    };
    let path = std::path::Path::new(endpoint);
    if !path.is_absolute() || endpoint.as_bytes().contains(&0) || endpoint.len() > 100 {
        return Err(io::Error::other("filesystem socket required"));
    }
    super::private_directory(
        path.parent()
            .ok_or_else(|| io::Error::other("socket parent"))?,
        server,
    )?;
    let before = std::fs::symlink_metadata(path)?;
    if !before.file_type().is_socket() || before.uid() != *uid || before.mode() & 0o777 != 0o600 {
        return Err(io::Error::other("private socket required"));
    }
    let stream = tokio::net::UnixStream::connect(path).await?;
    let after = std::fs::symlink_metadata(path)?;
    if stream.peer_cred()?.uid() != *uid
        || before.ino() != after.ino()
        || before.dev() != after.dev()
    {
        return Err(io::Error::other("socket peer or binding changed"));
    }
    Ok(stream)
}
impl Drop for Listener {
    fn drop(&mut self) {
        if let Ok(m) = std::fs::symlink_metadata(&self.path)
            && m.file_type().is_socket()
            && m.ino() == self.inode
            && m.dev() == self.device
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
