//! Raw HTTP/2 client framing for the owned-lab protocol-split attacks.
//!
//! Defense-only. This is a hand-encoded HTTP/2 client (its own framer, like the
//! Go `cmd/redteam/h2raw.go`) so the port controls the exact SETTINGS, the
//! connection WINDOW_UPDATE, the HEADERS priority, and the pseudo-header order
//! on the wire — the knobs the "protocol-split at the h2 layer" evasions turn.
//! [`H2Profile::GoTransport`] additionally models the frame layout Go's
//! `golang.org/x/net/http2` Transport (v0.57.0) wrote for the oracle's `doH2`
//! paths. It constructs bytes only; the live TLS/h2 transport is a recorded
//! gap. Every builder output is fed through the runtime's own passive H2
//! capture so the emitted fingerprint is what the detector would observe.
//!
//! Defense-only target restriction: `:authority` is caller-supplied but
//! [`H2Request::new`] accepts only loopback hosts or names under the reserved
//! `.test` TLD (RFC 6761), optionally with a port.
use crate::wire::rit::rit_token;

/// HTTP/2 connection preface a client sends before its first SETTINGS.
pub const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// One SETTINGS pair in wire order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    /// Unsigned SETTINGS identifier.
    pub id: u16,
    /// Unsigned SETTINGS value.
    pub value: u32,
}

/// The four Akamai h2-fingerprint dimensions as a named profile, mirroring the
/// Go h2raw profiles (R6 SETTINGS, R11 flow-control, R14 priority, ceiling).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H2Profile {
    /// Fully coherent Chrome h2: coherent SETTINGS, real window, priority set.
    Chrome,
    /// R6 `h2-settings-split`: library SETTINGS (no HEADER_TABLE_SIZE), no window.
    SettingsSplit,
    /// R11 `h2-flow-control`: coherent SETTINGS but a 1 GiB connection window.
    FlowControlSplit,
    /// R14 `h2-priority-absent`: coherent SETTINGS + real window, no priority.
    PriorityAbsent,
    /// Go `x/net/http2` Transport (oracle `doH2`/`doH2Spec`): its own SETTINGS,
    /// a 1 GiB connection window, no priority and the a,m,p,s pseudo order.
    GoTransport,
}

const CHROME_SETTINGS: &[Setting] = &[
    Setting {
        id: 1,
        value: 65536,
    },
    Setting { id: 2, value: 0 },
    Setting {
        id: 4,
        value: 6291456,
    },
    Setting {
        id: 6,
        value: 262144,
    },
];
const LIBRARY_SETTINGS: &[Setting] = &[
    Setting { id: 2, value: 0 },
    Setting {
        id: 4,
        value: 4194304,
    },
    Setting {
        id: 5,
        value: 16384,
    },
    Setting {
        id: 6,
        value: 10485760,
    },
];
/// x/net/http2 v0.57.0 `&http2.Transport{}` initial SETTINGS (push off, 4 MiB
/// stream window, 16 KiB max read frame, 10 MiB header list). An unset
/// `MaxReadFrameSize` is clipped to 16384 (`http2/config.go`), so these equal
/// `LIBRARY_SETTINGS`: the oracle's `h2SettingsSplit` copied Go's own SETTINGS.
const GO_TRANSPORT_SETTINGS: &[Setting] = LIBRARY_SETTINGS;
const CHROME_WINDOW: u32 = 15663105;
const ONE_GIB: u32 = 1_073_741_824;

impl H2Profile {
    /// SETTINGS pairs this profile writes, in wire order.
    pub fn settings(self) -> &'static [Setting] {
        match self {
            H2Profile::SettingsSplit => LIBRARY_SETTINGS,
            H2Profile::GoTransport => GO_TRANSPORT_SETTINGS,
            H2Profile::Chrome | H2Profile::FlowControlSplit | H2Profile::PriorityAbsent => {
                CHROME_SETTINGS
            }
        }
    }
    /// Connection-level WINDOW_UPDATE increment (0 means send none).
    pub fn window_update(self) -> u32 {
        match self {
            H2Profile::SettingsSplit => 0,
            H2Profile::FlowControlSplit | H2Profile::GoTransport => ONE_GIB,
            H2Profile::Chrome | H2Profile::PriorityAbsent => CHROME_WINDOW,
        }
    }
    /// Whether the HEADERS frame carries Chrome's priority (excl=1,dep=0,w=255).
    pub fn send_priority(self) -> bool {
        matches!(self, H2Profile::Chrome)
    }
    /// Pseudo-header wire order: the raw framer writes Chrome's m,a,s,p; the
    /// Go Transport writes a,m,p,s.
    pub fn pseudo_order(self) -> [&'static str; 4] {
        match self {
            H2Profile::GoTransport => [":authority", ":method", ":path", ":scheme"],
            _ => [":method", ":authority", ":scheme", ":path"],
        }
    }
}

/// SETTINGS_MAX_FRAME_SIZE initial value (RFC 9113 6.5.2). The framer never
/// emits a larger frame payload: DATA is split and header blocks continue in
/// CONTINUATION frames, since no SETTINGS from the peer is read here.
pub const MAX_FRAME_SIZE: usize = 16_384;

/// A refused raw-h2 request. Carries no request bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H2Error(pub &'static str);
impl std::fmt::Display for H2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for H2Error {}

/// Accept only owned-lab authorities: a loopback IP literal, `localhost`, or a
/// name under the reserved `.test` TLD, each with an optional numeric port.
fn owned_authority(authority: &str) -> bool {
    let host = if let Some(rest) = authority.strip_prefix('[') {
        // [v6]:port or [v6]
        match rest.split_once(']') {
            Some((host, "")) => host,
            Some((host, port)) => match port.strip_prefix(':') {
                Some(port) if valid_port(port) => host,
                _ => return false,
            },
            None => return false,
        }
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => {
                if !valid_port(port) {
                    return false;
                }
                host
            }
            Some(_) => return false, // bare IPv6 must be bracketed
            None => authority,
        }
    };
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return ip.is_loopback();
    }
    let host = host.to_ascii_lowercase();
    if host == "localhost" {
        return true;
    }
    let Some(label) = host.strip_suffix(".test") else {
        return false;
    };
    !label.is_empty()
        && label.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}
fn valid_port(port: &str) -> bool {
    !port.is_empty()
        && port.len() <= 5
        && port.bytes().all(|b| b.is_ascii_digit())
        && port.parse::<u16>().is_ok()
}

/// A single raw-h2 request to the owned runtime.
pub struct H2Request<'a> {
    profile: H2Profile,
    method: &'a str,
    authority: &'a str,
    path: &'a str,
    scheme: &'a str,
    /// Ordered regular headers (lowercase names, HTTP/2 requires it).
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl<'a> H2Request<'a> {
    /// Build a request in the profile's pseudo-header order. Refuses any
    /// `:authority` that is not an owned-lab name (loopback or `.test`).
    pub fn new(
        profile: H2Profile,
        method: &'a str,
        authority: &'a str,
        path: &'a str,
    ) -> Result<Self, H2Error> {
        if !owned_authority(authority) {
            return Err(H2Error(
                "h2 :authority must be loopback or a reserved .test name",
            ));
        }
        Ok(Self {
            profile,
            method,
            authority,
            path,
            scheme: "https",
            headers: Vec::new(),
            body: Vec::new(),
        })
    }
    /// Append one regular header, preserving wire order.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_ascii_lowercase(), value.into()));
        self
    }
    /// Attach a request body (the stream then ends on a DATA frame).
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
    /// Sign the request's body with a RIT token for the given seed/counter.
    pub fn rit_signed(self, seed: &[u8], sid: &str, n: u64, tb: u64) -> Self {
        let token = rit_token(seed, sid, n, tb, &self.body);
        self.header("x-hm-token", &token)
            .header("x-hm-n", &n.to_string())
            .header("x-hm-tb", &tb.to_string())
    }

    /// Encode the full client byte stream: preface, SETTINGS, optional
    /// WINDOW_UPDATE, then the HEADERS (+ DATA) for stream 1.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(PREFACE);

        // SETTINGS frame (non-ack) on stream 0.
        let mut settings = Vec::new();
        for s in self.profile.settings() {
            settings.extend_from_slice(&s.id.to_be_bytes());
            settings.extend_from_slice(&s.value.to_be_bytes());
        }
        frame(&mut out, 4, 0, 0, &settings);

        // Connection WINDOW_UPDATE between SETTINGS and the first HEADERS.
        let window = self.profile.window_update();
        if window > 0 {
            frame(&mut out, 8, 0, 0, &window.to_be_bytes());
        }

        // HPACK header block: pseudo-headers in profile order, then regular headers.
        let mut block = Vec::new();
        for pseudo in self.profile.pseudo_order() {
            let value = match pseudo {
                ":method" => self.method,
                ":authority" => self.authority,
                ":scheme" => self.scheme,
                _ => self.path,
            };
            hpack_literal(&mut block, pseudo.as_bytes(), value.as_bytes());
        }
        for (name, value) in &self.headers {
            hpack_literal(&mut block, name.as_bytes(), value.as_bytes());
        }

        let mut headers_payload = Vec::new();
        let mut flags = 0;
        if self.profile.send_priority() {
            flags |= 0x20; // PRIORITY present in HEADERS
            // exclusive(1 bit) + dependency(31 bits) = stream 0, exclusive.
            headers_payload.extend_from_slice(&0x8000_0000u32.to_be_bytes());
            headers_payload.push(255); // weight byte
        }
        if self.body.is_empty() {
            flags |= 0x01; // END_STREAM on HEADERS when there is no body
        }
        // The HEADERS frame carries as much of the block as fits; the rest
        // follows in CONTINUATION frames, END_HEADERS on the last one.
        let first = (MAX_FRAME_SIZE - headers_payload.len()).min(block.len());
        headers_payload.extend_from_slice(&block[..first]);
        let mut rest = block[first..].chunks(MAX_FRAME_SIZE).peekable();
        if rest.peek().is_none() {
            flags |= 0x04; // END_HEADERS
        }
        frame(&mut out, 1, flags, 1, &headers_payload);
        while let Some(chunk) = rest.next() {
            let end = if rest.peek().is_none() { 0x04 } else { 0 };
            frame(&mut out, 9, end, 1, chunk); // CONTINUATION
        }

        // DATA split at SETTINGS_MAX_FRAME_SIZE, END_STREAM on the last frame.
        let mut chunks = self.body.chunks(MAX_FRAME_SIZE).peekable();
        while let Some(chunk) = chunks.next() {
            let end = if chunks.peek().is_none() { 0x01 } else { 0 };
            frame(&mut out, 0, end, 1, chunk);
        }
        out
    }
}

/// Append one HTTP/2 frame (9-byte header + payload). Callers split payloads
/// at [`MAX_FRAME_SIZE`], so the 24-bit length field can never be truncated.
fn frame(out: &mut Vec<u8>, kind: u8, flags: u8, stream: u32, payload: &[u8]) {
    assert!(
        payload.len() <= MAX_FRAME_SIZE,
        "frame payload exceeds SETTINGS_MAX_FRAME_SIZE"
    );
    let len = u32::try_from(payload.len()).expect("bounded by MAX_FRAME_SIZE");
    out.extend_from_slice(&len.to_be_bytes()[1..]); // 24-bit length
    out.push(kind);
    out.push(flags);
    out.extend_from_slice(&(stream & 0x7fff_ffff).to_be_bytes());
    out.extend_from_slice(payload);
}

/// Encode one HPACK "literal header field without indexing, new name" entry,
/// Huffman off. The runtime's HPACK decoder reads this back verbatim.
fn hpack_literal(out: &mut Vec<u8>, name: &[u8], value: &[u8]) {
    out.push(0x00); // literal without indexing, name not indexed
    hpack_string(out, name);
    hpack_string(out, value);
}
fn hpack_string(out: &mut Vec<u8>, bytes: &[u8]) {
    hpack_integer(out, bytes.len(), 7); // H flag (top bit) stays 0 = no Huffman
    out.extend_from_slice(bytes);
}
fn hpack_integer(out: &mut Vec<u8>, value: usize, prefix: u8) {
    let max = (1usize << prefix) - 1;
    if value < max {
        out.push(value as u8);
        return;
    }
    out.push(max as u8);
    let mut rest = value - max;
    while rest >= 128 {
        out.push(((rest % 128) + 128) as u8);
        rest /= 128;
    }
    out.push(rest as u8);
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_runtime::capture::http2::Capture;
    use humanymous_runtime::network::http2::Fingerprint;

    fn observe(req: &H2Request) -> Fingerprint {
        let mut capture = Capture::default();
        capture
            .feed(&req.encode())
            .expect("runtime H2 capture accepts the frames");
        capture
            .request(1)
            .expect("stream 1 request completed")
            .fingerprint()
            .clone()
    }

    #[test]
    fn chrome_profile_reproduces_the_full_browser_h2_fingerprint() {
        let req = H2Request::new(H2Profile::Chrome, "POST", "owned.test", "/api/collect")
            .unwrap()
            .header("user-agent", "chrome")
            .body(b"{}".to_vec());
        let fp = observe(&req);
        assert_eq!(fp.pseudo_order, vec!["m", "a", "s", "p"]);
        assert_eq!(fp.engine(), "chrome");
        assert_eq!(fp.window_update, CHROME_WINDOW);
        assert_eq!(fp.headers_priority, "1:0:255");
        assert!(fp.has_setting(1)); // HEADER_TABLE_SIZE present
        assert_eq!(fp.header_order, vec!["user-agent".to_string()]);
    }

    #[test]
    fn settings_split_drops_header_table_size_and_sends_no_window() {
        let fp = observe(
            &H2Request::new(
                H2Profile::SettingsSplit,
                "GET",
                "owned.test",
                "/api/session",
            )
            .unwrap()
            .header("user-agent", "chrome"),
        );
        // Chrome pseudo-order (engine reads chrome) but a library SETTINGS set.
        assert_eq!(fp.pseudo_order, vec!["m", "a", "s", "p"]);
        assert!(!fp.has_setting(1));
        assert_eq!(fp.window_update, 0);
        assert_eq!(fp.headers_priority, "");
    }

    #[test]
    fn flow_control_split_opens_a_gigabyte_window_with_coherent_settings() {
        let fp = observe(
            &H2Request::new(
                H2Profile::FlowControlSplit,
                "GET",
                "owned.test",
                "/api/session",
            )
            .unwrap(),
        );
        assert!(fp.has_setting(1));
        assert_eq!(fp.window_update, ONE_GIB);
        assert_eq!(fp.headers_priority, "");
    }

    #[test]
    fn priority_absent_keeps_the_real_window_but_omits_headers_priority() {
        let fp = observe(
            &H2Request::new(
                H2Profile::PriorityAbsent,
                "GET",
                "owned.test",
                "/api/session",
            )
            .unwrap(),
        );
        assert!(fp.has_setting(1));
        assert_eq!(fp.window_update, CHROME_WINDOW);
        assert_eq!(fp.headers_priority, "");
    }

    #[test]
    fn rit_signing_appends_the_token_headers_in_wire_order() {
        let req = H2Request::new(H2Profile::Chrome, "POST", "owned.test", "/api/collect")
            .unwrap()
            .body(b"{\"signals\":[]}".to_vec())
            .rit_signed(b"seed-bytes-000000000000000000000", "sid", 3, 5);
        let fp = observe(&req);
        assert_eq!(
            fp.header_order,
            vec![
                "x-hm-token".to_string(),
                "x-hm-n".to_string(),
                "x-hm-tb".to_string()
            ]
        );
    }

    #[test]
    fn hpack_integer_handles_lengths_above_the_prefix_max() {
        // A 200-byte value forces multi-byte HPACK length; the decoder must agree.
        let long = "a".repeat(200);
        let req = H2Request::new(H2Profile::Chrome, "POST", "owned.test", "/api/collect")
            .unwrap()
            .header("user-agent", &long)
            .body(b"{}".to_vec());
        let mut capture = Capture::default();
        capture
            .feed(&req.encode())
            .expect("long header round-trips through HPACK");
        let request = capture.request(1).expect("stream 1 completed");
        let value = request
            .fields()
            .iter()
            .find(|(n, _)| n == b"user-agent")
            .map(|(_, v)| v.clone())
            .expect("user-agent captured");
        assert_eq!(value.len(), 200);
    }

    #[test]
    fn go_transport_writes_amps_pseudo_order_and_its_own_settings_and_window() {
        let fp = observe(
            &H2Request::new(H2Profile::GoTransport, "GET", "owned.test", "/api/session").unwrap(),
        );
        assert_eq!(fp.pseudo_order, vec!["a", "m", "p", "s"]);
        assert_eq!(fp.engine(), "unknown");
        // Captured from `&http2.Transport{}` (x/net v0.57.0): SETTINGS
        // 2=0, 4=4194304, 5=16384, 6=10485760, then WINDOW_UPDATE 1073741824.
        let settings: Vec<(u16, u32)> = fp.settings.iter().map(|s| (s.id, s.value)).collect();
        assert_eq!(
            settings,
            vec![(2, 0), (4, 4194304), (5, 16384), (6, 10485760)]
        );
        assert_eq!(fp.window_update, 1_073_741_824);
        assert_eq!(fp.headers_priority, "");
    }

    #[test]
    fn authority_is_restricted_to_loopback_and_reserved_test_names() {
        for ok in [
            "owned.test",
            "lab.owned.test:8443",
            "127.0.0.1",
            "127.0.0.1:8443",
            "[::1]:443",
            "[::1]",
            "localhost:8443",
        ] {
            assert!(
                H2Request::new(H2Profile::Chrome, "GET", ok, "/").is_ok(),
                "{ok}"
            );
        }
        for refused in [
            "example.com",
            "example.com:443",
            "93.184.216.34",
            "10.0.0.1:8443",
            "[2001:db8::1]:443",
            "::1",
            ".test",
            "owned.test.example.com",
            "owned.test:99999",
            "owned.test:",
            "owned.test:+443",
            "[::1]:+443",
            "",
        ] {
            assert!(
                H2Request::new(H2Profile::Chrome, "GET", refused, "/").is_err(),
                "{refused}"
            );
        }
    }

    /// Walk the encoded stream after the preface and return (type, flags, len).
    fn frames(bytes: &[u8]) -> Vec<(u8, u8, usize)> {
        let mut rest = &bytes[PREFACE.len()..];
        let mut out = Vec::new();
        while rest.len() >= 9 {
            let len =
                (usize::from(rest[0]) << 16) | (usize::from(rest[1]) << 8) | usize::from(rest[2]);
            out.push((rest[3], rest[4], len));
            rest = &rest[9 + len..];
        }
        assert!(rest.is_empty());
        out
    }

    #[test]
    fn large_bodies_split_into_max_frame_size_data_frames() {
        let body = vec![b'x'; 2 * MAX_FRAME_SIZE + 100];
        let req = H2Request::new(H2Profile::Chrome, "POST", "owned.test", "/api/collect")
            .unwrap()
            .body(body);
        let encoded = req.encode();
        let data: Vec<_> = frames(&encoded).into_iter().filter(|f| f.0 == 0).collect();
        assert_eq!(
            data,
            vec![
                (0, 0, MAX_FRAME_SIZE),
                (0, 0, MAX_FRAME_SIZE),
                (0, 0x01, 100)
            ]
        );
        // The runtime capture (16 KiB frame budget) accepts every frame.
        let mut capture = Capture::default();
        capture
            .feed(&encoded)
            .expect("split DATA within frame budget");
        assert!(capture.request(1).is_some());
    }

    #[test]
    fn oversized_header_blocks_continue_in_continuation_frames() {
        let long = "v".repeat(MAX_FRAME_SIZE + 500);
        let req = H2Request::new(H2Profile::Chrome, "GET", "owned.test", "/")
            .unwrap()
            .header("x-long", &long);
        let encoded = req.encode();
        let kinds: Vec<_> = frames(&encoded)
            .into_iter()
            .filter(|f| f.0 == 1 || f.0 == 9)
            .collect();
        assert_eq!(kinds.len(), 2);
        // HEADERS: priority + END_STREAM, no END_HEADERS; full-size payload.
        assert_eq!(kinds[0], (1, 0x20 | 0x01, MAX_FRAME_SIZE));
        // CONTINUATION ends the block.
        assert_eq!(kinds[1].0, 9);
        assert_eq!(kinds[1].1, 0x04);
        assert!(kinds.iter().all(|f| f.2 <= MAX_FRAME_SIZE));
    }
}
