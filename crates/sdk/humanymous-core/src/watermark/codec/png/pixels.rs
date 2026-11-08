//! PNG RGB LSB carrier stub. Fail-closed until alpha/resource proofs land.

use crate::watermark::{Error, frame::Limits};

/// Pixel LSB issuance is not admitted in this L2 slice.
pub fn encode(_source: &[u8], _carrier: &[u8], _limits: Limits) -> Result<Vec<u8>, Error> {
    Err(Error::InvalidConfiguration)
}

/// Pixel LSB recovery is not admitted in this L2 slice.
pub fn recover(_source: &[u8], _limits: Limits) -> Result<Vec<u8>, Error> {
    Err(Error::InvalidConfiguration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsb_channel_stays_fail_closed() {
        let limits = Limits::new(64, 64, 1).unwrap();
        assert_eq!(
            encode(b"\x89PNG\r\n\x1a\n", b"x", limits),
            Err(Error::InvalidConfiguration)
        );
        assert_eq!(
            recover(b"\x89PNG\r\n\x1a\n", limits),
            Err(Error::InvalidConfiguration)
        );
    }
}
