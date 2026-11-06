use crate::{
    configuration::{Configuration, Peer},
    decision::NativePeer,
};
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
    pub fn bind(config: &Configuration) -> io::Result<Self> {
        let path = PathBuf::from(&config.endpoint);
        let Peer::Unix { uid } = config.peer else {
            return Err(io::Error::other("Unix principal required"));
        };
        if !path.is_absolute()
            || config.endpoint.as_bytes().contains(&0)
            || config.endpoint.len() > 100
            || path.file_name().is_none()
            || rustix::process::geteuid().as_raw() != uid
        {
            return Err(io::Error::other("invalid filesystem socket or owner"));
        }
        super::private_directory(
            path.parent()
                .ok_or_else(|| io::Error::other("socket parent"))?,
            &config.peer,
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
