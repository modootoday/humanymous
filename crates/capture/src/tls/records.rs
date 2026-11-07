//! Record-boundary accounting without retaining encrypted payloads.

#[derive(Default)]
pub(super) struct RecordBoundary {
    header: [u8; 5],
    used: usize,
    remaining: usize,
}

impl RecordBoundary {
    pub(super) fn read_limit(&self, requested: usize) -> usize {
        requested.min(if self.used < 5 {
            5 - self.used
        } else {
            self.remaining
        })
    }

    pub(super) fn is_idle(&self) -> bool {
        self.used == 0
    }

    pub(super) fn feed(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            if self.used < 5 {
                let count = (5 - self.used).min(bytes.len());
                self.header[self.used..self.used + count].copy_from_slice(&bytes[..count]);
                self.used += count;
                bytes = &bytes[count..];
                if self.used < 5 {
                    return;
                }
                self.remaining = usize::from(u16::from_be_bytes([self.header[3], self.header[4]]));
            }
            let count = self.remaining.min(bytes.len());
            self.remaining -= count;
            bytes = &bytes[count..];
            if self.remaining == 0 {
                self.used = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fragment_stops_exactly_at_record_boundaries() {
        let wire = [23, 3, 3, 0, 3, 1, 2, 3, 23, 3, 3, 0, 1, 4];
        for chunk in 1..=wire.len() {
            let mut records = RecordBoundary::default();
            let mut consumed = 0;
            while consumed < wire.len() {
                let count = records.read_limit(chunk).min(wire.len() - consumed);
                assert!(count > 0);
                records.feed(&wire[consumed..consumed + count]);
                consumed += count;
                assert_eq!(records.is_idle(), consumed == 8 || consumed == wire.len());
            }
        }
    }

    #[test]
    fn incomplete_headers_and_payloads_remain_non_idle() {
        let wire = [23, 3, 3, 0, 3, 1, 2, 3];
        for end in 1..wire.len() {
            let mut records = RecordBoundary::default();
            records.feed(&wire[..end]);
            assert!(!records.is_idle());
        }
        let mut records = RecordBoundary::default();
        records.feed(&[23, 3, 3, 0, 0]);
        assert!(records.is_idle());
        assert_eq!(records.read_limit(4096), 5);
        assert!(std::mem::size_of::<RecordBoundary>() < 64);
    }
}
