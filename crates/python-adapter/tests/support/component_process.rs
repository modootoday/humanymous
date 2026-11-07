//! Bound the known WSGI process group; never claim arbitrary descendant control.
use humanymous_test_support::process::{ManagedChild, capture_with_stderr};
use std::{
    fs,
    io::Read,
    os::unix::{fs::MetadataExt, process::CommandExt},
    path::Path,
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};

fn members(bundle: &Path) -> Result<Vec<(u32, u32)>, String> {
    let owner = fs::metadata("/proc/self")
        .map_err(|_| "actor identity unavailable")?
        .uid();
    let marker = format!("HUMANYMOUS_PYTHON_MODULE_DIR={}/module", bundle.display());
    let expected_group = fs::read_to_string(bundle.join("actual-group.private"))
        .ok()
        .map(|value| value.trim().parse::<u32>())
        .transpose()
        .map_err(|_| "invalid owned group ledger")?;
    let mut owned = Vec::new();
    for entry in fs::read_dir("/proc").map_err(|_| "process inventory unavailable")? {
        let entry = entry.map_err(|_| "process inventory entry unavailable")?;
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if fs::metadata(entry.path()).is_ok_and(|metadata| metadata.uid() == owner) {
            let stat = match fs::read_to_string(entry.path().join("stat")) {
                Ok(stat) => stat,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return Err("actor process identity unavailable".into()),
            };
            let fields: Vec<_> = stat
                .rsplit_once(") ")
                .ok_or("invalid process identity")?
                .1
                .split_whitespace()
                .collect();
            let group = fields
                .get(2)
                .ok_or("missing process group")?
                .parse()
                .map_err(|_| "invalid process group")?;
            let environment = fs::read(entry.path().join("environ"));
            let marked = environment.as_ref().is_ok_and(|bytes| {
                bytes
                    .split(|byte| *byte == 0)
                    .any(|value| value == marker.as_bytes())
            });
            if expected_group == Some(group) && !marked {
                if environment
                    .as_ref()
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
                {
                    continue;
                }
                return Err("recorded group member lacks required task identity".into());
            }
            if marked {
                owned.push((pid, group));
            }
        }
    }
    Ok(owned)
}

pub(super) fn cleanup(bundle: &Path) -> Result<(), String> {
    let ledger = bundle.join("actual-group.private");
    let group = match fs::read_to_string(&ledger) {
        Ok(value) => value
            .trim()
            .parse::<u32>()
            .map_err(|_| "invalid owned group ledger")?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return if members(bundle)?.is_empty() {
                Ok(())
            } else {
                Err("task process exists without owned group ledger".into())
            };
        }
        Err(_) => return Err("owned group ledger unavailable".into()),
    };
    if group <= 1 {
        return Err("invalid owned group".into());
    }
    let remaining = members(bundle)?;
    if remaining.is_empty() {
        return Ok(());
    }
    if remaining.iter().any(|(_, observed)| *observed != group) {
        return Err("task process escaped recorded group; cleanup not established".into());
    }
    // A live member binds the recorded group to this private task marker and UID.
    // This is the known harness topology, not protection against a hostile child.
    let mut kill = Command::new("/usr/bin/kill");
    kill.args(["--signal", "KILL", "--", &format!("-{group}")]);
    let result = capture_with_stderr(&mut kill, b"", Duration::from_secs(5), 4096)
        .map_err(|_| "owned group cleanup command failed")?;
    if !result.status.success() && !members(bundle)?.is_empty() {
        return Err("owned process group refused cleanup".into());
    }
    let until = Instant::now() + Duration::from_secs(5);
    while !members(bundle)?.is_empty() {
        if Instant::now() >= until {
            return Err("owned process cleanup deadline".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

pub(super) fn actual(
    command: &mut Command,
    bundle: &Path,
) -> Result<(std::process::ExitStatus, Vec<u8>), String> {
    command.process_group(0);
    let (mut child, input, output) =
        ManagedChild::spawn_duplex(command).map_err(|_| "actual harness launch failed")?;
    drop(input);
    fs::write(bundle.join("actual-group.private"), child.id().to_string())
        .map_err(|_| "owned group ledger write failed")?;
    let (send, receive) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .take((4 << 20) + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "actual output read failed")
            .and_then(|_| {
                if bytes.len() > 4 << 20 {
                    Err("actual output overflow")
                } else {
                    Ok(bytes)
                }
            });
        let _ = send.send(result);
    });
    let status = child.wait(Duration::from_secs(180));
    let cleaned = cleanup(bundle);
    let stopped = child.stop();
    let bytes = receive
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "actual stdout closure deadline")?;
    reader.join().map_err(|_| "actual output worker panicked")?;
    cleaned?;
    stopped.map_err(|_| "actual harness reap failed")?;
    Ok((
        status.map_err(|_| "actual harness deadline")?,
        bytes.map_err(str::to_owned)?,
    ))
}
