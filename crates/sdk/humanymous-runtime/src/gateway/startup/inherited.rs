//! Supervisor-inherited witness pipes for the shipped runtime binary (C3).
//! The audit-owned client codec and key pinning are reused unchanged.
use super::ExternalWitness;
use crate::audit::{
    checkpoint::{WitnessBootstrap, service::WitnessServiceClient},
    log::WitnessDurability,
};
use crate::gateway::configuration::{Configuration, WitnessInputDurability};
use std::io;

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "witness input refused")
}

/// Complete the pinned witness handshake over inherited descriptors before any
/// custody, WAL or unlock credential is touched. Absent input returns `None`;
/// every failure refuses, and nothing falls back to writer-only startup.
pub fn inherited_witness(config: &Configuration) -> io::Result<Option<ExternalWitness>> {
    let Some(input) = &config.audit.witness_input else {
        return Ok(None);
    };
    if !config.audit.witness_input_valid() {
        return Err(refused());
    }
    let (read, write) = platform::open_pair(input.read_fd, input.write_fd)?;
    let bootstrap = match config.audit.trusted_head.as_ref() {
        Some(minimum) => WitnessBootstrap::Pinned {
            minimum,
            consistency_proof: &input.consistency_proof,
        },
        None => WitnessBootstrap::CreateNew,
    };
    let client = WitnessServiceClient::from_ready(
        io::BufReader::new(read),
        write,
        input.witness_public,
        input.writer_public,
        config.audit.node_id.clone(),
        bootstrap,
    )?;
    let durability = match input.durability {
        WitnessInputDurability::FileAndDirectory => WitnessDurability::FileAndDirectory,
        WitnessInputDurability::FileSyncedNamespaceUnconfirmed => WitnessDurability::FileSynced,
    };
    ExternalWitness::new(Box::new(client), input.witness_public, durability).map(Some)
}

#[cfg(unix)]
mod platform {
    use super::refused;
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
    use std::{
        fs::{File, OpenOptions},
        io,
        os::unix::fs::{FileTypeExt, OpenOptionsExt},
    };

    pub(super) fn open_pair(read_fd: u32, write_fd: u32) -> io::Result<(File, File)> {
        Ok((open(read_fd, false)?, open(write_fd, true)?))
    }

    /// `unsafe_code` is forbidden, so the inherited number is reopened through
    /// the descriptor filesystem rather than adopted. The open itself never
    /// waits for a FIFO peer; blocking mode is restored for framed I/O. A
    /// silent or vanished peer surfaces only at I/O or the supervisor deadline.
    fn open(fd: u32, write: bool) -> io::Result<File> {
        #[cfg(target_os = "linux")]
        access_mode(fd, write)?;
        #[cfg(target_os = "linux")]
        let path = format!("/proc/self/fd/{fd}");
        #[cfg(not(target_os = "linux"))]
        let path = format!("/dev/fd/{fd}");
        let file = OpenOptions::new()
            .read(!write)
            .write(write)
            .custom_flags(OFlags::NONBLOCK.bits() as i32)
            .open(path)
            .map_err(|_| refused())?;
        if !file
            .metadata()
            .map_err(|_| refused())?
            .file_type()
            .is_fifo()
        {
            return Err(refused());
        }
        let flags = fcntl_getfl(&file).map_err(|_| refused())?;
        fcntl_setfl(&file, flags - OFlags::NONBLOCK).map_err(|_| refused())?;
        Ok(file)
    }

    /// Linux reopening ignores the original direction, so check it first.
    #[cfg(target_os = "linux")]
    fn access_mode(fd: u32, write: bool) -> io::Result<()> {
        let info =
            std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}")).map_err(|_| refused())?;
        let flags = info
            .lines()
            .find_map(|line| line.strip_prefix("flags:"))
            .and_then(|value| u32::from_str_radix(value.trim(), 8).ok())
            .ok_or_else(refused)?;
        // O_ACCMODE values are fixed by the Linux ABI: 0 read-only, 1 write-only.
        if flags & 3 == u32::from(write) {
            Ok(())
        } else {
            Err(refused())
        }
    }
}

#[cfg(not(unix))]
mod platform {
    use std::{fs::File, io};
    pub(super) fn open_pair(_: u32, _: u32) -> io::Result<(File, File)> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "inherited witness input is unsupported on this platform",
        ))
    }
}
