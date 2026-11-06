/// Order-preserving ClientHello observations. Opaque ALPN names remain bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClientHello {
    /// Legacy handshake version.
    pub legacy_version: u16,
    /// Offered supported versions, including GREASE.
    pub supported_versions: Vec<u16>,
    /// Cipher suites in wire order.
    pub cipher_suites: Vec<u16>,
    /// Extension types in wire order.
    pub extensions: Vec<u16>,
    /// Supported groups in wire order.
    pub curves: Vec<u16>,
    /// EC point formats widened to unsigned integers.
    pub point_formats: Vec<u16>,
    /// Signature algorithms in wire order.
    pub signature_algorithms: Vec<u16>,
    /// Opaque ALPN protocol names without UTF-8 normalization.
    pub alpn: Vec<Vec<u8>>,
    /// Nonempty server_name extension, preserving the frozen observation rule.
    pub sni_present: bool,
}

/// The observed prefix cannot supply the required handshake fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError;
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("clienthello: truncated")
    }
}
impl std::error::Error for ParseError {}

struct Cursor<'a>(&'a [u8]);
impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], ParseError> {
        let value = self.0.get(..length).ok_or(ParseError)?;
        self.0 = &self.0[length..];
        Ok(value)
    }
    fn vec8(&mut self) -> Result<&'a [u8], ParseError> {
        let length = usize::from(self.take(1)?[0]);
        self.take(length)
    }
    fn vec16(&mut self) -> Result<&'a [u8], ParseError> {
        let bytes = self.take(2)?;
        self.take(usize::from(u16::from_be_bytes([bytes[0], bytes[1]])))
    }
}
fn words(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .collect()
}

/// Observe one handshake prefix, preserving legacy tolerance of declared lengths
/// beyond captured bytes and incomplete trailing extensions. This is an
/// observation parser, not TLS handshake validation or capture completeness.
pub fn parse_client_hello(bytes: &[u8]) -> Result<ClientHello, ParseError> {
    if bytes.len() < 4 || bytes[0] != 1 {
        return Err(ParseError);
    }
    let length =
        (usize::from(bytes[1]) << 16) | (usize::from(bytes[2]) << 8) | usize::from(bytes[3]);
    let mut cursor = Cursor(&bytes[4..4 + length.min(bytes.len() - 4)]);
    let version = cursor.take(2)?;
    let mut hello = ClientHello {
        legacy_version: u16::from_be_bytes([version[0], version[1]]),
        ..Default::default()
    };
    cursor.take(32)?;
    cursor.vec8()?;
    hello.cipher_suites = words(cursor.vec16()?);
    cursor.vec8()?;
    if cursor.0.is_empty() {
        return Ok(hello);
    }
    let mut extensions = Cursor(cursor.vec16()?);
    while extensions.0.len() >= 4 {
        let kind = extensions.take(2)?;
        let kind = u16::from_be_bytes([kind[0], kind[1]]);
        let Ok(data) = extensions.vec16() else {
            break;
        };
        hello.extensions.push(kind);
        match kind {
            0 => hello.sni_present = !data.is_empty(),
            10 | 13 => {
                if let Ok(value) = Cursor(data).vec16() {
                    if kind == 10 {
                        hello.curves = words(value);
                    } else {
                        hello.signature_algorithms = words(value);
                    }
                }
            }
            11 => {
                if let Ok(value) = Cursor(data).vec8() {
                    hello.point_formats = value.iter().map(|&v| u16::from(v)).collect();
                }
            }
            43 => {
                if let Ok(value) = Cursor(data).vec8() {
                    hello.supported_versions = words(value);
                }
            }
            16 => {
                hello.alpn.clear();
                if let Ok(list) = Cursor(data).vec16() {
                    let mut list = Cursor(list);
                    while !list.0.is_empty() {
                        let Ok(name) = list.vec8() else {
                            break;
                        };
                        hello.alpn.push(name.to_vec());
                    }
                }
            }
            _ => {}
        }
    }
    Ok(hello)
}

/// Observe a single TLS handshake record prefix. Record reassembly and strict
/// completeness validation belong to the connection capture owner.
pub fn parse_record(bytes: &[u8]) -> Result<ClientHello, ParseError> {
    if bytes.len() < 5 || bytes[0] != 22 {
        return Err(ParseError);
    }
    let length = usize::from(u16::from_be_bytes([bytes[3], bytes[4]]));
    parse_client_hello(&bytes[5..5 + length.min(bytes.len() - 5)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_truncation_never_advances_beyond_the_capture() {
        for bytes in [vec![], vec![0], vec![0, 2, 1], vec![255, 255]] {
            let mut cursor = Cursor(&bytes);
            assert!(cursor.take(bytes.len() + 1).is_err());
            assert_eq!(cursor.0, bytes);
        }
        assert_eq!(words(&[1, 2, 3]), [258]);
        assert!(Cursor(&[2, 1]).vec8().is_err());
        assert!(Cursor(&[0, 2, 1]).vec16().is_err());
    }

    #[test]
    fn a_capture_is_an_observation_and_does_not_validate_a_tls_handshake() {
        let mut handshake = vec![1, 0, 0, 255, 3, 3];
        handshake.extend([0; 32]);
        handshake.extend([0, 0, 2, 0x13, 1, 1, 0]);
        let hello = parse_client_hello(&handshake).unwrap();
        assert_eq!(hello.cipher_suites, [0x1301]);
        assert!(hello.extensions.is_empty());
        handshake[0] = 2;
        assert_eq!(parse_client_hello(&handshake), Err(ParseError));
    }
}
