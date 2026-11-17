//! Raw HTTP/2 client framing for the owned-lab protocol-split attacks.
//!
//! Defense-only. This is a hand-encoded HTTP/2 client (its own framer, like the
//! Go `cmd/redteam/h2raw.go`) so the port controls the exact SETTINGS, the
//! connection WINDOW_UPDATE, the HEADERS priority, and the pseudo-header order
//! on the wire — the knobs the "protocol-split at the h2 layer" evasions turn.
//! It constructs bytes only; the live TLS/h2 transport is a recorded gap. Every
//! builder output is fed through the runtime's own passive H2 capture so the
//! emitted fingerprint is exactly what the detector would observe.
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
const CHROME_WINDOW: u32 = 15663105;
const ONE_GIB: u32 = 1_073_741_824;

impl H2Profile {
    /// SETTINGS pairs this profile writes, in wire order.
    pub fn settings(self) -> &'static [Setting] {
        match self {
            H2Profile::SettingsSplit => LIBRARY_SETTINGS,
            _ => CHROME_SETTINGS,
        }
    }
    /// Connection-level WINDOW_UPDATE increment (0 means send none).
    pub fn window_update(self) -> u32 {
        match self {
            H2Profile::SettingsSplit => 0,
            H2Profile::FlowControlSplit => ONE_GIB,
            H2Profile::Chrome | H2Profile::PriorityAbsent => CHROME_WINDOW,
        }
    }
    /// Whether the HEADERS frame carries Chrome's priority (excl=1,dep=0,w=255).
    pub fn send_priority(self) -> bool {
        matches!(self, H2Profile::Chrome)
    }
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
    /// Build a request in Chrome's m,a,s,p pseudo-header order.
    pub fn new(profile: H2Profile, method: &'a str, authority: &'a str, path: &'a str) -> Self {
        Self {
            profile,
            method,
            authority,
            path,
            scheme: "https",
            headers: Vec::new(),
            body: Vec::new(),
        }
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

        // HPACK header block: pseudo-headers m,a,s,p, then regular headers.
        let mut block = Vec::new();
        hpack_literal(&mut block, b":method", self.method.as_bytes());
        hpack_literal(&mut block, b":authority", self.authority.as_bytes());
        hpack_literal(&mut block, b":scheme", self.scheme.as_bytes());
        hpack_literal(&mut block, b":path", self.path.as_bytes());
        for (name, value) in &self.headers {
            hpack_literal(&mut block, name.as_bytes(), value.as_bytes());
        }

        let mut headers_payload = Vec::new();
        let mut flags = 0x04; // END_HEADERS
        if self.profile.send_priority() {
            flags |= 0x20; // PRIORITY present in HEADERS
            // exclusive(1 bit) + dependency(31 bits) = stream 0, exclusive.
            headers_payload.extend_from_slice(&0x8000_0000u32.to_be_bytes());
            headers_payload.push(255); // weight byte
        }
        if self.body.is_empty() {
            flags |= 0x01; // END_STREAM on HEADERS when there is no body
        }
        headers_payload.extend_from_slice(&block);
        frame(&mut out, 1, flags, 1, &headers_payload);

        if !self.body.is_empty() {
            frame(&mut out, 0, 0x01, 1, &self.body); // DATA, END_STREAM
        }
        out
    }
}

/// Append one HTTP/2 frame (9-byte header + payload).
fn frame(out: &mut Vec<u8>, kind: u8, flags: u8, stream: u32, payload: &[u8]) {
    let len = u32::try_from(payload.len()).expect("frame payload under 16 MiB");
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
        let fp = observe(&H2Request::new(
            H2Profile::FlowControlSplit,
            "GET",
            "owned.test",
            "/api/session",
        ));
        assert!(fp.has_setting(1));
        assert_eq!(fp.window_update, ONE_GIB);
        assert_eq!(fp.headers_priority, "");
    }

    #[test]
    fn priority_absent_keeps_the_real_window_but_omits_headers_priority() {
        let fp = observe(&H2Request::new(
            H2Profile::PriorityAbsent,
            "GET",
            "owned.test",
            "/api/session",
        ));
        assert!(fp.has_setting(1));
        assert_eq!(fp.window_update, CHROME_WINDOW);
        assert_eq!(fp.headers_priority, "");
    }

    #[test]
    fn rit_signing_appends_the_token_headers_in_wire_order() {
        let req = H2Request::new(H2Profile::Chrome, "POST", "owned.test", "/api/collect")
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
}
