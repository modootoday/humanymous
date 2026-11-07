use humanymous_network::tls::{ClientHello, parse_client_hello};

const MAX_HELLO: usize = 65_536;
const MAX_RECORD: usize = 18_432;
const MAX_RECORDS: usize = 128;

/// Terminal observation failure; contains no captured personal data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    /// Invalid TLS record or nested ClientHello vector.
    Malformed,
    /// Fixed byte/count budget exceeded.
    Budget,
    /// EOF before complete capture.
    Truncated,
}
impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TLS capture {self:?}")
    }
}
impl std::error::Error for CaptureError {}

/// Bounded first-ClientHello assembler. Completion is framing evidence only;
/// only [`super::Connection`] qualifies the actual TLS handshake.
#[derive(Default)]
pub struct ClientHelloCapture {
    header: [u8; 5],
    header_used: usize,
    remaining: usize,
    records: usize,
    handshake: Vec<u8>,
    expected: Option<usize>,
    complete: bool,
    failed: Option<CaptureError>,
}
impl ClientHelloCapture {
    /// Observe exactly the ingress bytes, preserving arbitrary fragmentation.
    /// Failure is sticky; no later valid input repairs an invalid epoch.
    pub fn feed(&mut self, input: &[u8]) -> Result<(), CaptureError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        let result = self.feed_inner(input);
        if let Err(error) = result {
            self.failed = Some(error);
            self.handshake.clear();
        }
        result
    }

    fn feed_inner(&mut self, mut input: &[u8]) -> Result<(), CaptureError> {
        while !input.is_empty() && !self.complete {
            if self.header_used < 5 {
                let count = (5 - self.header_used).min(input.len());
                self.header[self.header_used..self.header_used + count]
                    .copy_from_slice(&input[..count]);
                self.header_used += count;
                input = &input[count..];
                if self.header_used < 5 {
                    break;
                }
                self.records += 1;
                self.remaining = usize::from(u16::from_be_bytes([self.header[3], self.header[4]]));
                if self.records > MAX_RECORDS || self.remaining > MAX_RECORD {
                    return Err(CaptureError::Budget);
                }
                if self.header[0] != 22
                    || self.header[1] != 3
                    || !matches!(self.header[2], 1..=3)
                    || self.remaining == 0
                {
                    return Err(CaptureError::Malformed);
                }
            }
            let wanted = self.expected.unwrap_or(4) - self.handshake.len();
            let count = wanted.min(self.remaining).min(input.len());
            self.handshake.extend_from_slice(&input[..count]);
            input = &input[count..];
            self.remaining -= count;
            if self.handshake.len() == 4 && self.expected.is_none() {
                if self.handshake[0] != 1 {
                    return Err(CaptureError::Malformed);
                }
                let length = (usize::from(self.handshake[1]) << 16)
                    | (usize::from(self.handshake[2]) << 8)
                    | usize::from(self.handshake[3]);
                if length > MAX_HELLO - 4 {
                    return Err(CaptureError::Budget);
                }
                self.expected = Some(length + 4);
            }
            if self.expected == Some(self.handshake.len()) {
                // The first client flight is exactly a ClientHello. Reject
                // hidden trailing messages in its record in this profile.
                if self.remaining != 0 {
                    return Err(CaptureError::Malformed);
                }
                validate(&self.handshake[4..])?;
                self.complete = true;
            }
            if self.remaining == 0 {
                self.header_used = 0;
            }
        }
        Ok(())
    }

    /// True only for a complete strictly framed first ClientHello.
    pub fn is_complete(&self) -> bool {
        self.complete && self.failed.is_none()
    }

    /// Consume framing evidence. Does not claim cryptographic TLS success.
    pub fn finish(self) -> Result<ClientHello, CaptureError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if !self.complete {
            return Err(CaptureError::Truncated);
        }
        parse_client_hello(&self.handshake).map_err(|_| CaptureError::Malformed)
    }
}

struct Cursor<'a>(&'a [u8]);
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], CaptureError> {
        let value = self.0.get(..count).ok_or(CaptureError::Malformed)?;
        self.0 = &self.0[count..];
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, CaptureError> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16, CaptureError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
    fn vec8(&mut self) -> Result<&'a [u8], CaptureError> {
        let count = usize::from(self.byte()?);
        self.take(count)
    }
    fn vec16(&mut self) -> Result<&'a [u8], CaptureError> {
        let count = usize::from(self.word()?);
        self.take(count)
    }
    fn end(self) -> Result<(), CaptureError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(CaptureError::Malformed)
        }
    }
}
fn words(bytes: &[u8]) -> Result<(), CaptureError> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        Err(CaptureError::Malformed)
    } else {
        Ok(())
    }
}
fn validate(bytes: &[u8]) -> Result<(), CaptureError> {
    let mut hello = Cursor(bytes);
    hello.take(34)?;
    if hello.vec8()?.len() > 32 {
        return Err(CaptureError::Malformed);
    }
    words(hello.vec16()?)?;
    if hello.vec8()?.is_empty() {
        return Err(CaptureError::Malformed);
    }
    // TLS1.2 permits an omitted extension vector.
    if hello.0.is_empty() {
        return Ok(());
    }
    let mut extensions = Cursor(hello.vec16()?);
    hello.end()?;
    let mut seen = std::collections::BTreeSet::new();
    while !extensions.0.is_empty() {
        let kind = extensions.word()?;
        if !seen.insert(kind) {
            return Err(CaptureError::Malformed);
        }
        let mut data = Cursor(extensions.vec16()?);
        match kind {
            10 | 13 => words(data.vec16()?)?,
            11 => {
                if data.vec8()?.is_empty() {
                    return Err(CaptureError::Malformed);
                }
            }
            43 => words(data.vec8()?)?,
            16 => {
                let mut list = Cursor(data.vec16()?);
                if list.0.is_empty() {
                    return Err(CaptureError::Malformed);
                }
                while !list.0.is_empty() {
                    if list.vec8()?.is_empty() {
                        return Err(CaptureError::Malformed);
                    }
                }
            }
            0 => {
                let mut list = Cursor(data.vec16()?);
                if list.0.is_empty() {
                    return Err(CaptureError::Malformed);
                }
                let mut types = std::collections::BTreeSet::new();
                while !list.0.is_empty() {
                    if !types.insert(list.byte()?) || list.vec16()?.is_empty() {
                        return Err(CaptureError::Malformed);
                    }
                }
            }
            _ => {
                data.0 = &[];
            }
        }
        data.end()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
