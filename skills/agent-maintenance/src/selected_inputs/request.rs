use super::{
    Clock, ContentRead, FrozenSelection, Input, Limits, PreparationReceipt, ReadBudget, ReadStatus,
    Root, SelectionError, SystemClock, files,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::{Component, Path},
    time::Duration,
};

pub const MAX_REQUEST_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u8,
    roots: Vec<Root>,
    files: Vec<Input>,
    input_selection_complete: bool,
}

/// Diagnostics contain codes/positions, never serde's raw offending JSON values.
#[derive(Debug)]
pub enum RequestError {
    TooLarge,
    InvalidPath,
    InvalidJson { line: usize, column: usize },
    UnsupportedVersion,
    Selection(SelectionError),
    File(io::ErrorKind),
    Changed,
    IncompleteRead(ReadStatus),
    CommandBudget,
}
impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "selected-input request: {self:?}")
    }
}
impl std::error::Error for RequestError {}

pub fn decode_request(bytes: &[u8]) -> Result<FrozenSelection, RequestError> {
    if bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err(RequestError::TooLarge);
    }
    let request: Request =
        serde_json::from_slice(bytes).map_err(|e| RequestError::InvalidJson {
            line: e.line(),
            column: e.column(),
        })?;
    if request.schema_version != 1 {
        return Err(RequestError::UnsupportedVersion);
    }
    FrozenSelection::new(
        request.roots,
        request.files,
        request.input_selection_complete,
        &Limits::default(),
    )
    .map_err(RequestError::Selection)
}

#[derive(Serialize)]
pub struct CliReceipt {
    pub schema: &'static str,
    pub request_read: ContentRead,
    pub observation: PreparationReceipt,
    pub command_actual_read_bytes: u64,
    pub command_elapsed_micros: u128,
    pub command_budget_complete: bool,
}

struct Capture {
    file: File,
    bytes: Vec<u8>,
}
impl Read for Capture {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = self.file.read(output)?;
        self.bytes.extend_from_slice(&output[..count]);
        Ok(count)
    }
}

fn remaining_limits(bytes: u64, elapsed: Duration) -> Result<Limits, RequestError> {
    let limits = Limits::default();
    let max_total_read_bytes = limits
        .max_total_read_bytes
        .checked_sub(bytes)
        .filter(|n| *n > 0)
        .ok_or(RequestError::CommandBudget)?;
    limits
        .max_elapsed
        .checked_sub(elapsed)
        .filter(|n| !n.is_zero())
        .ok_or(RequestError::CommandBudget)?;
    Ok(Limits {
        max_total_read_bytes,
        ..limits
    })
}

pub(crate) fn capture_request_file(
    path: &Path,
    clock: &mut SystemClock,
) -> Result<(Vec<u8>, ContentRead), RequestError> {
    if path
        .to_str()
        .is_none_or(|s| s.len() > 4096 || s.chars().any(char::is_control))
        || path.components().any(|c| matches!(c, Component::ParentDir))
    {
        return Err(RequestError::InvalidPath);
    }
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| RequestError::File(e.kind()))?
            .join(path)
    };
    let before = files::checked_path(&path, true).map_err(|e| RequestError::File(e.kind()))?;
    if before.len() > MAX_REQUEST_BYTES {
        return Err(RequestError::TooLarge);
    }
    let modified = before
        .modified()
        .map_err(|e| RequestError::File(e.kind()))?;
    let file = File::open(&path).map_err(|e| RequestError::File(e.kind()))?;
    let opened = file.metadata().map_err(|e| RequestError::File(e.kind()))?;
    if !opened.is_file()
        || opened.len() != before.len()
        || opened
            .modified()
            .map_err(|e| RequestError::File(e.kind()))?
            != modified
    {
        return Err(RequestError::Changed);
    }
    let mut captured = Capture {
        file,
        bytes: Vec::new(),
    };
    let mut request_budget = ReadBudget::new(Limits {
        max_file_bytes: MAX_REQUEST_BYTES,
        max_total_read_bytes: MAX_REQUEST_BYTES + 1,
        ..Limits::default()
    })
    .map_err(RequestError::Selection)?;
    let request_read = request_budget.read(&mut captured, clock);
    if !request_read.complete() {
        return Err(RequestError::IncompleteRead(request_read.status));
    }
    let after = files::checked_path(&path, true).map_err(|e| RequestError::File(e.kind()))?;
    let handle = captured
        .file
        .metadata()
        .map_err(|e| RequestError::File(e.kind()))?;
    if after.len() != before.len()
        || handle.len() != before.len()
        || after.modified().map_err(|e| RequestError::File(e.kind()))? != modified
        || handle
            .modified()
            .map_err(|e| RequestError::File(e.kind()))?
            != modified
    {
        return Err(RequestError::Changed);
    }
    Ok((captured.bytes, request_read))
}

pub fn observe_request_file(path: &Path) -> Result<CliReceipt, RequestError> {
    let mut clock = SystemClock::default();
    let (bytes, request_read) = capture_request_file(path, &mut clock)?;
    let selection = decode_request(&bytes)?;
    let limits = remaining_limits(request_read.actual_read_bytes, clock.elapsed())?;
    let observation = files::observe_with_clock(&selection, limits, &mut clock)
        .map_err(RequestError::Selection)?;
    let bytes = request_read
        .actual_read_bytes
        .checked_add(observation.actual_read_bytes)
        .ok_or(RequestError::CommandBudget)?;
    let elapsed = clock.elapsed();
    let defaults = Limits::default();
    Ok(CliReceipt {
        schema: "humanymous.selected-inputs-cli.v1",
        command_budget_complete: bytes <= defaults.max_total_read_bytes
            && elapsed < defaults.max_elapsed,
        command_actual_read_bytes: bytes,
        command_elapsed_micros: elapsed.as_micros(),
        request_read,
        observation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> serde_json::Value {
        serde_json::json!({"schema_version":1,"roots":[{"id":"r","path":std::env::temp_dir()}],"files":[{"root":"r","path":"selected"}],"input_selection_complete":true})
    }
    #[test]
    fn request_schema_requires_explicit_fields_and_rejects_unknown_or_duplicate_keys() {
        assert!(decode_request(&serde_json::to_vec(&valid()).unwrap()).is_ok());
        let mut extra = valid();
        extra["unexpected"] = "not-a-profile".into();
        assert!(matches!(
            decode_request(&serde_json::to_vec(&extra).unwrap()),
            Err(RequestError::InvalidJson { .. })
        ));
        assert!(matches!(
            decode_request(br#"{"schema_version":1,"schema_version":1}"#),
            Err(RequestError::InvalidJson { .. })
        ));
        let mut missing = valid();
        missing
            .as_object_mut()
            .unwrap()
            .remove("input_selection_complete");
        assert!(matches!(
            decode_request(&serde_json::to_vec(&missing).unwrap()),
            Err(RequestError::InvalidJson { .. })
        ));
    }
    #[test]
    fn request_size_and_version_are_bounded_without_parser_value_disclosure() {
        assert!(matches!(
            decode_request(&vec![b' '; MAX_REQUEST_BYTES as usize + 1]),
            Err(RequestError::TooLarge)
        ));
        let mut version = valid();
        version["schema_version"] = 2.into();
        assert!(matches!(
            decode_request(&serde_json::to_vec(&version).unwrap()),
            Err(RequestError::UnsupportedVersion)
        ));
        let error = decode_request(br#"{"schema_version":"PRIVATE_SENTINEL"}"#)
            .err()
            .unwrap();
        assert!(!error.to_string().contains("PRIVATE_SENTINEL"));
    }
    #[test]
    fn request_reuses_selection_escape_and_count_guards() {
        let mut escaped = valid();
        escaped["files"][0]["path"] = "../outside".into();
        assert!(matches!(
            decode_request(&serde_json::to_vec(&escaped).unwrap()),
            Err(RequestError::Selection(SelectionError::InvalidPath))
        ));
        let mut count = valid();
        count["files"] = serde_json::json!(
            (0..5001)
                .map(|i| serde_json::json!({"root":"r","path":format!("{i}")}))
                .collect::<Vec<_>>()
        );
        assert!(matches!(
            decode_request(&serde_json::to_vec(&count).unwrap()),
            Err(RequestError::Selection(SelectionError::CountLimit))
        ));
    }
    #[test]
    fn request_reads_and_time_are_deducted_from_the_command_budget() {
        let l = remaining_limits(7, Duration::from_secs(2)).unwrap();
        assert_eq!(
            l.max_total_read_bytes,
            Limits::default().max_total_read_bytes - 7
        );
        // The original clock continues; its absolute deadline is unchanged.
        assert_eq!(l.max_elapsed, Duration::from_secs(30));
        assert!(matches!(
            remaining_limits(Limits::default().max_total_read_bytes, Duration::ZERO),
            Err(RequestError::CommandBudget)
        ));
        assert!(matches!(
            remaining_limits(0, Duration::from_secs(30)),
            Err(RequestError::CommandBudget)
        ));
    }
}
