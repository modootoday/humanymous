use crate::{Error, Request};

/// Supported Apache input filter modes for the bounded complete-body profile.
#[derive(Clone, Copy)]
pub enum ReadMode {
    /// Consume at most the requested byte count.
    Bytes,
    /// Consume through the first newline, bounded by the requested byte count.
    Line,
    /// Inspect without advancing; native AP_MODE_SPECULATIVE mapping.
    Speculative,
}

/// Owned body replay that retains request admission and cancellation checks.
/// This is an input mechanism, never an authorization ticket. The native owner
/// must consume its bound authority response before installing the input filter.
pub struct Replay {
    request: Request,
    position: usize,
}
impl Replay {
    /// Take a complete request without copying its body or releasing admission.
    pub fn new(request: Request) -> Result<Self, Error> {
        request.body()?;
        Ok(Self {
            request,
            position: 0,
        })
    }

    /// Return the next complete-body slice, checking original request liveness.
    /// An empty result means EOF only when `eof` is true. Zero-length reads do
    /// not consume bytes, create EOF prematurely or restart the request budget.
    pub fn read(&mut self, mode: ReadMode, maximum: usize) -> Result<&[u8], Error> {
        let body = self.request.body()?;
        let available = &body[self.position..];
        let mut length = available.len().min(maximum);
        if matches!(mode, ReadMode::Line)
            && let Some(index) = available[..length].iter().position(|byte| *byte == b'\n')
        {
            length = index + 1;
        }
        let start = self.position;
        if !matches!(mode, ReadMode::Speculative) {
            self.position += length;
        }
        Ok(&body[start..start + length])
    }

    /// Only full consumption is EOF; speculative reads never consume input.
    pub fn eof(&self) -> Result<bool, Error> {
        Ok(self.position == self.request.body()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Child, Mpm};
    use std::time::Duration;

    fn fixture(body: &[u8]) -> (Child, Replay) {
        let child = Child::initialize(1, Mpm::Prefork, 1).unwrap();
        let mut request = child
            .begin(1, true, body.len(), 8, Duration::from_secs(1))
            .unwrap();
        request.append(body).unwrap();
        request.finish().unwrap();
        (child, Replay::new(request).unwrap())
    }

    #[test]
    fn speculative_reads_do_not_consume_or_release_request() {
        let (child, mut replay) = fixture(&[0, 255]);
        assert_eq!(replay.read(ReadMode::Speculative, 8).unwrap(), [0, 255]);
        assert_eq!(replay.read(ReadMode::Speculative, 1).unwrap(), [0]);
        assert!(!replay.eof().unwrap());
        assert_eq!(replay.read(ReadMode::Bytes, 0).unwrap(), []);
        assert_eq!(replay.read(ReadMode::Bytes, 1).unwrap(), [0]);
        assert_eq!(replay.read(ReadMode::Bytes, 8).unwrap(), [255]);
        assert!(replay.eof().unwrap());
        assert!(child.begin(1, true, 0, 8, Duration::from_secs(1)).is_err());
        drop(replay);
        assert!(child.begin(1, true, 0, 8, Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn line_mode_preserves_binary_and_line_boundaries() {
        let (_, mut replay) = fixture(b"a\n\0\xff\nb");
        assert_eq!(replay.read(ReadMode::Line, 8).unwrap(), b"a\n");
        assert_eq!(replay.read(ReadMode::Line, 1).unwrap(), b"\0");
        assert_eq!(replay.read(ReadMode::Line, 8).unwrap(), b"\xff\n");
        assert_eq!(replay.read(ReadMode::Line, 8).unwrap(), b"b");
        assert!(replay.eof().unwrap());
    }

    #[test]
    fn cleanup_revocation_blocks_even_previously_complete_replay() {
        let (_, mut replay) = fixture(b"data");
        let cleanup = replay.request.cancellation();
        cleanup.cancel();
        assert!(replay.read(ReadMode::Bytes, 8).is_err());
        assert!(replay.eof().is_err());
    }
}
