//! Passive bounded-memory frame accounting; HTTP serving owns validation.

/// Connection-scoped frozen Rapid Reset/CONTINUATION/control-frame observer.
/// It consumes only framing metadata and at most four payload bytes; payloads
/// are never buffered wholesale and the supplied byte stream is never mutated.
#[derive(Debug)]
pub struct FrameMonitor {
    preface_remaining: usize,
    header: [u8; 9],
    header_used: usize,
    payload_remaining: usize,
    prefix: [u8; 4],
    prefix_used: usize,
    resets: u16,
    continuations: u16,
    protocol_errors: u16,
    control: u16,
    abuse: Option<&'static str>,
}
impl Default for FrameMonitor {
    fn default() -> Self {
        Self {
            preface_remaining: 24,
            header: [0; 9],
            header_used: 0,
            payload_remaining: 0,
            prefix: [0; 4],
            prefix_used: 0,
            resets: 0,
            continuations: 0,
            protocol_errors: 0,
            control: 0,
            abuse: None,
        }
    }
}

impl FrameMonitor {
    /// First threshold-crossing signal, fixed for the rest of the connection.
    pub fn abuse(&self) -> Option<&'static str> {
        self.abuse
    }

    /// Observe an arbitrary decrypted chunk. Counts only completed frames and
    /// preserves state across any header/payload/preface fragmentation.
    pub fn feed(&mut self, mut input: &[u8]) {
        if self.abuse.is_some() {
            return;
        }
        let skipped = self.preface_remaining.min(input.len());
        input = &input[skipped..];
        self.preface_remaining -= skipped;
        while !input.is_empty() {
            if self.header_used < 9 {
                let count = (9 - self.header_used).min(input.len());
                self.header[self.header_used..self.header_used + count]
                    .copy_from_slice(&input[..count]);
                self.header_used += count;
                input = &input[count..];
                if self.header_used < 9 {
                    return;
                }
                self.payload_remaining = self.length();
                self.prefix_used = 0;
            }
            let count = self.payload_remaining.min(input.len());
            let prefix_count = (4 - self.prefix_used).min(count);
            self.prefix[self.prefix_used..self.prefix_used + prefix_count]
                .copy_from_slice(&input[..prefix_count]);
            self.prefix_used += prefix_count;
            self.payload_remaining -= count;
            input = &input[count..];
            if self.payload_remaining != 0 {
                return;
            }
            self.classify();
            self.header_used = 0;
            if self.abuse.is_some() {
                return;
            }
        }
    }
    fn length(&self) -> usize {
        (usize::from(self.header[0]) << 16)
            | (usize::from(self.header[1]) << 8)
            | usize::from(self.header[2])
    }
    fn classify(&mut self) {
        match self.header[3] {
            3 => {
                self.resets += 1;
                if self.resets > 15 {
                    self.abuse = Some("l5.h2dos.rapid_reset");
                }
            }
            9 => {
                self.continuations += 1;
                if self.continuations > 20 {
                    self.abuse = Some("l5.h2dos.continuation_flood");
                }
            }
            8 => {
                self.control += 1;
                if self.length() == 4 && u32::from_be_bytes(self.prefix) & 0x7fff_ffff == 0 {
                    self.protocol_errors += 1;
                }
            }
            2 => {
                self.control += 1;
                if self.length() != 5 {
                    self.protocol_errors += 1;
                }
            }
            4 | 6 => self.control += 1,
            _ => {}
        }
        if self.protocol_errors > 10 || self.control > 300 {
            self.abuse = Some("l5.h2dos.rapid_reset");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
        let len = payload.len();
        let mut bytes = vec![
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
            kind,
            0,
            0,
            0,
            0,
            1,
        ];
        bytes.extend(payload);
        bytes
    }
    #[test]
    fn thresholds_are_strict_and_partial_frames_are_not_counted() {
        for (kind, payload, limit, signal) in [
            (3, vec![0; 4], 15, "l5.h2dos.rapid_reset"),
            (9, vec![], 20, "l5.h2dos.continuation_flood"),
            (8, vec![0; 4], 10, "l5.h2dos.rapid_reset"),
            (2, vec![], 10, "l5.h2dos.rapid_reset"),
            (6, vec![0; 8], 300, "l5.h2dos.rapid_reset"),
        ] {
            let mut monitor = FrameMonitor::default();
            monitor.feed(&[0; 24]);
            let bytes = frame(kind, &payload);
            for _ in 0..limit {
                monitor.feed(&bytes);
                assert_eq!(monitor.abuse(), None);
            }
            for byte in &bytes[..bytes.len() - 1] {
                monitor.feed(&[*byte]);
                assert_eq!(monitor.abuse(), None);
            }
            monitor.feed(&bytes[bytes.len() - 1..]);
            assert_eq!(monitor.abuse(), Some(signal));
            monitor.feed(&frame(9, &[]));
            assert_eq!(monitor.abuse(), Some(signal));
        }
    }
    #[test]
    fn huge_partial_payload_has_fixed_observation_memory_and_no_early_verdict() {
        let mut monitor = FrameMonitor::default();
        monitor.feed(&[0; 24]);
        monitor.feed(&[255, 255, 255, 3, 0, 0, 0, 0, 1]);
        for _ in 0..1000 {
            monitor.feed(&[0; 1024]);
        }
        assert_eq!(monitor.resets, 0);
        assert_eq!(monitor.abuse(), None);
        assert_eq!(monitor.prefix_used, 4);
        assert!(std::mem::size_of::<FrameMonitor>() < 128);
    }
}
