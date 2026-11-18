//! TLS ClientHello construction for the owned-lab parrot clients.
//!
//! Defense-only. Each [`Profile`] is structurally modelled on the ClientHello
//! the Go oracle (`cmd/redteam`, `cmd/tlsparrot`, repo 981d25c) put on the
//! wire: the utls v1.8.2 presets `HelloChrome_Auto` (= `HelloChrome_133`),
//! `HelloChrome_100` and `HelloFirefox_Auto` (= `HelloFirefox_120`), and Go
//! 1.25 stock `crypto/tls` for `doStock`. Cipher suites, extension list and
//! declaration order, supported groups, signature algorithms, key-share groups
//! and lengths, ALPN, ALPS codepoint, ECH GREASE and BoringSSL-style padding
//! follow those presets, and every output is accepted by the runtime's own
//! ClientHello capture. It only *constructs* bytes; see the module notes on
//! live handshakes.
//!
//! Known differences from what utls/Go emit (recorded gaps, not claimed):
//! - key-share, ECH `enc`/payload, random and session-id bytes are constant
//!   filler (`0x42`/`0x11`/`0x22`), not real key material, so no handshake can
//!   complete with these bytes;
//! - GREASE values are fixed (`0x0a0a`/`0x1a1a`); utls draws them per
//!   connection (JA3/JA4 filter GREASE, so fingerprints are unaffected);
//! - `HelloChrome_133` shuffles its extensions per connection with a CSPRNG;
//!   here the preset declaration order is used unless [`ClientHelloBuilder::permute`]
//!   applies a deterministic seeded shuffle with the same fixed positions
//!   (GREASE stays in place), so the permutation is reproducible, not utls's;
//! - the ECH GREASE payload length is the first utls candidate (128+16 for
//!   Chrome, 223+16 for Firefox); utls picks a candidate at random;
//! - the stock profile reproduces a Go 1.25.3 `crypto/tls` hello with
//!   `NextProtos: ["http/1.1"]` as captured from that toolchain; other Go
//!   versions differ (for example in default groups).
//!
//! No arbitrary-target capability exists here: the SNI is fixed to the
//! reserved `owned.test` name and the bytes are exercised against the owned
//! runtime's capture path in tests.

/// Fixed GREASE codepoints. A second distinct value is used for the trailing
/// extension so the capture's duplicate-type guard stays quiet (utls does the
/// same by xoring the second value when the two random picks collide).
const GREASE_A: u16 = 0x0a0a;
const GREASE_B: u16 = 0x1a1a;

/// Extension codepoints (wire-order significant for the fingerprint).
pub mod ext {
    /// server_name.
    pub const SERVER_NAME: u16 = 0;
    /// status_request.
    pub const STATUS_REQUEST: u16 = 5;
    /// supported_groups.
    pub const SUPPORTED_GROUPS: u16 = 10;
    /// ec_point_formats.
    pub const EC_POINT_FORMATS: u16 = 11;
    /// signature_algorithms.
    pub const SIGNATURE_ALGORITHMS: u16 = 13;
    /// application_layer_protocol_negotiation.
    pub const ALPN: u16 = 16;
    /// signed_certificate_timestamp.
    pub const SCT: u16 = 18;
    /// padding.
    pub const PADDING: u16 = 21;
    /// extended_master_secret.
    pub const EXTENDED_MASTER_SECRET: u16 = 23;
    /// compress_certificate.
    pub const COMPRESS_CERTIFICATE: u16 = 27;
    /// record_size_limit.
    pub const RECORD_SIZE_LIMIT: u16 = 28;
    /// delegated_credentials.
    pub const DELEGATED_CREDENTIALS: u16 = 34;
    /// session_ticket.
    pub const SESSION_TICKET: u16 = 35;
    /// supported_versions.
    pub const SUPPORTED_VERSIONS: u16 = 43;
    /// psk_key_exchange_modes.
    pub const PSK_KEY_EXCHANGE_MODES: u16 = 45;
    /// signature_algorithms_cert.
    pub const SIGNATURE_ALGORITHMS_CERT: u16 = 50;
    /// key_share.
    pub const KEY_SHARE: u16 = 51;
    /// ALPS application_settings, old codepoint (utls HelloChrome_100).
    pub const APPLICATION_SETTINGS: u16 = 17513;
    /// ALPS application_settings, new codepoint (utls HelloChrome_133).
    pub const APPLICATION_SETTINGS_NEW: u16 = 17613;
    /// encrypted_client_hello (GREASE form in every profile here).
    pub const ENCRYPTED_CLIENT_HELLO: u16 = 0xfe0d;
    /// renegotiation_info.
    pub const RENEGOTIATION_INFO: u16 = 0xff01;
}

/// X25519MLKEM768 hybrid post-quantum group.
pub const GROUP_X25519_MLKEM768: u16 = 0x11ec;
const X25519: u16 = 0x001d;
const P256: u16 = 0x0017;
const P384: u16 = 0x0018;
const P521: u16 = 0x0019;
const FFDHE2048: u16 = 0x0100;
const FFDHE3072: u16 = 0x0101;

/// utls Chrome cipher suites in wire order after the leading GREASE
/// (identical for `HelloChrome_100` and `HelloChrome_133`).
const CHROME_CIPHERS: &[u16] = &[
    0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc013, 0xc014, 0x009c,
    0x009d, 0x002f, 0x0035,
];
/// utls `HelloFirefox_120` cipher suites in wire order (no GREASE).
const FIREFOX_CIPHERS: &[u16] = &[
    0x1301, 0x1303, 0x1302, 0xc02b, 0xc02f, 0xcca9, 0xcca8, 0xc02c, 0xc030, 0xc00a, 0xc009, 0xc013,
    0xc014, 0x009c, 0x009d, 0x002f, 0x0035,
];
/// Go 1.25.3 stock `crypto/tls` client cipher suites in wire order.
const STOCK_CIPHERS: &[u16] = &[
    0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc009, 0xc013, 0xc00a, 0xc014, 0x1301, 0x1302,
    0x1303,
];
const CHROME_SIG_ALGS: &[u16] = &[
    0x0403, 0x0804, 0x0401, 0x0503, 0x0805, 0x0501, 0x0806, 0x0601,
];
const FIREFOX_SIG_ALGS: &[u16] = &[
    0x0403, 0x0503, 0x0603, 0x0804, 0x0805, 0x0806, 0x0401, 0x0501, 0x0601, 0x0203, 0x0201,
];
const FIREFOX_DELEGATED_SIG_ALGS: &[u16] = &[0x0403, 0x0503, 0x0603, 0x0203];
const STOCK_SIG_ALGS: &[u16] = &[
    0x0804, 0x0403, 0x0807, 0x0805, 0x0806, 0x0401, 0x0501, 0x0601, 0x0503, 0x0603,
];
const STOCK_SIG_ALGS_CERT: &[u16] = &[
    0x0804, 0x0403, 0x0807, 0x0805, 0x0806, 0x0401, 0x0501, 0x0601, 0x0503, 0x0603, 0x0201, 0x0203,
];

/// Which oracle ClientHello the builder models.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// utls `HelloChrome_Auto` = `HelloChrome_133`: GREASE, X25519MLKEM768,
    /// new ALPS codepoint (17613), ECH GREASE, compress_certificate, no padding;
    /// utls shuffles its extensions per connection (see [`ClientHelloBuilder::permute`]).
    Chrome,
    /// utls `HelloChrome_100`: GREASE, no PQ group, old ALPS codepoint (17513),
    /// no ECH, BoringSSL padding, fixed extension order (the static parrot).
    Chrome100,
    /// utls `HelloFirefox_Auto` = `HelloFirefox_120`: no GREASE, ffdhe groups,
    /// delegated_credentials, record_size_limit, ECH GREASE, fixed order.
    Firefox,
    /// Go 1.25.3 stock `crypto/tls` as used by the oracle's `doStock`: no
    /// GREASE, no compress_certificate, ALPN `http/1.1` only.
    Stock,
}

impl Profile {
    /// The oracle preset this profile is modelled on.
    pub fn oracle_preset(self) -> &'static str {
        match self {
            Profile::Chrome => "utls v1.8.2 HelloChrome_Auto (HelloChrome_133)",
            Profile::Chrome100 => "utls v1.8.2 HelloChrome_100",
            Profile::Firefox => "utls v1.8.2 HelloFirefox_Auto (HelloFirefox_120)",
            Profile::Stock => "Go 1.25.3 crypto/tls (doStock, NextProtos http/1.1)",
        }
    }

    fn greased(self) -> bool {
        matches!(self, Profile::Chrome | Profile::Chrome100)
    }

    /// Whether the oracle preset permutes its extension order per connection.
    pub fn shuffles_extensions(self) -> bool {
        self == Profile::Chrome
    }
}

/// A single-extension deviation from the coherent Chrome parrot, isolating the
/// residual tell each Go attack targeted. Each mirrors a Go `strip*` helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// Drop ALPS application_settings, both codepoints (Go `stripALPS`).
    StripAlps,
    /// Drop the Encrypted Client Hello GREASE extension (Go `stripECH`).
    StripEch,
    /// Drop compress_certificate, ext 27 (Go `stripCertCompression`).
    StripCertCompression,
}

/// The ALPN list offered in the hello.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alpn {
    /// `h2, http/1.1`: the presets' own list (Go `doH2`/`doH2Spec`/raw h2).
    H2AndHttp11,
    /// `http/1.1` only: Go `forceHTTP11` on the `do()` paths and `doStock`.
    Http11Only,
}

/// A ClientHello builder. Random/session bytes are fixed for reproducibility;
/// they are not fingerprint-bearing fields.
pub struct ClientHelloBuilder {
    profile: Profile,
    mutations: Vec<Mutation>,
    alpn: Alpn,
    permutation: Option<u64>,
    sni: &'static str,
}

impl ClientHelloBuilder {
    /// Start from an oracle profile with its own ALPN list (`http/1.1` only
    /// for [`Profile::Stock`], `h2, http/1.1` otherwise).
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            mutations: Vec::new(),
            alpn: if profile == Profile::Stock {
                Alpn::Http11Only
            } else {
                Alpn::H2AndHttp11
            },
            permutation: None,
            sni: "owned.test",
        }
    }

    /// Apply a single-extension deviation (the residual tell under test).
    pub fn mutate(mut self, mutation: Mutation) -> Self {
        self.mutations.push(mutation);
        self
    }

    /// Offer `http/1.1` only, like the oracle's `forceHTTP11` on its HTTP/1.1
    /// request paths.
    pub fn force_http11(mut self) -> Self {
        self.alpn = Alpn::Http11Only;
        self
    }

    /// Model one connection of a shuffling preset: apply a deterministic
    /// seeded permutation to every extension except GREASE (and padding), as
    /// utls `ShuffleChromeTLSExtensions` does with a random seed. A no-op for
    /// profiles whose oracle preset keeps a fixed order.
    pub fn permute(mut self, seed: u64) -> Self {
        if self.profile.shuffles_extensions() {
            self.permutation = Some(seed);
        }
        self
    }

    fn has(&self, m: Mutation) -> bool {
        self.mutations.contains(&m)
    }

    fn groups(&self) -> Vec<u16> {
        match self.profile {
            Profile::Chrome => vec![GREASE_A, GROUP_X25519_MLKEM768, X25519, P256, P384],
            Profile::Chrome100 => vec![GREASE_A, X25519, P256, P384],
            Profile::Firefox => vec![X25519, P256, P384, P521, FFDHE2048, FFDHE3072],
            Profile::Stock => vec![GROUP_X25519_MLKEM768, X25519, P256, P384, P521],
        }
    }

    /// Key shares as (group, share length). Lengths follow the real encodings
    /// (GREASE 1 byte, X25519 32, P-256 uncompressed 65, X25519MLKEM768 1216).
    fn key_shares(&self) -> Vec<(u16, usize)> {
        match self.profile {
            Profile::Chrome => vec![(GREASE_A, 1), (GROUP_X25519_MLKEM768, 1216), (X25519, 32)],
            Profile::Chrome100 => vec![(GREASE_A, 1), (X25519, 32)],
            Profile::Firefox => vec![(X25519, 32), (P256, 65)],
            Profile::Stock => vec![(GROUP_X25519_MLKEM768, 1216), (X25519, 32)],
        }
    }

    /// The extension types in the oracle preset's declaration order.
    fn preset_extensions(&self) -> Vec<u16> {
        use ext::*;
        match self.profile {
            Profile::Chrome => vec![
                GREASE_A,
                SERVER_NAME,
                EXTENDED_MASTER_SECRET,
                RENEGOTIATION_INFO,
                SUPPORTED_GROUPS,
                EC_POINT_FORMATS,
                SESSION_TICKET,
                ALPN,
                STATUS_REQUEST,
                SIGNATURE_ALGORITHMS,
                SCT,
                KEY_SHARE,
                PSK_KEY_EXCHANGE_MODES,
                SUPPORTED_VERSIONS,
                COMPRESS_CERTIFICATE,
                APPLICATION_SETTINGS_NEW,
                ENCRYPTED_CLIENT_HELLO,
                GREASE_B,
            ],
            Profile::Chrome100 => vec![
                GREASE_A,
                SERVER_NAME,
                EXTENDED_MASTER_SECRET,
                RENEGOTIATION_INFO,
                SUPPORTED_GROUPS,
                EC_POINT_FORMATS,
                SESSION_TICKET,
                ALPN,
                STATUS_REQUEST,
                SIGNATURE_ALGORITHMS,
                SCT,
                KEY_SHARE,
                PSK_KEY_EXCHANGE_MODES,
                SUPPORTED_VERSIONS,
                COMPRESS_CERTIFICATE,
                APPLICATION_SETTINGS,
                GREASE_B,
                PADDING,
            ],
            Profile::Firefox => vec![
                SERVER_NAME,
                EXTENDED_MASTER_SECRET,
                RENEGOTIATION_INFO,
                SUPPORTED_GROUPS,
                EC_POINT_FORMATS,
                SESSION_TICKET,
                ALPN,
                STATUS_REQUEST,
                DELEGATED_CREDENTIALS,
                KEY_SHARE,
                SUPPORTED_VERSIONS,
                SIGNATURE_ALGORITHMS,
                PSK_KEY_EXCHANGE_MODES,
                RECORD_SIZE_LIMIT,
                ENCRYPTED_CLIENT_HELLO,
            ],
            Profile::Stock => vec![
                SERVER_NAME,
                EC_POINT_FORMATS,
                RENEGOTIATION_INFO,
                EXTENDED_MASTER_SECRET,
                SCT,
                STATUS_REQUEST,
                SUPPORTED_GROUPS,
                SIGNATURE_ALGORITHMS,
                SIGNATURE_ALGORITHMS_CERT,
                ALPN,
                SUPPORTED_VERSIONS,
                KEY_SHARE,
            ],
        }
    }

    /// The extension types, in wire order, for the selected profile/mutations.
    pub fn extension_plan(&self) -> Vec<u16> {
        let mut plan = self.preset_extensions();
        plan.retain(|&e| match e {
            ext::APPLICATION_SETTINGS | ext::APPLICATION_SETTINGS_NEW => {
                !self.has(Mutation::StripAlps)
            }
            ext::ENCRYPTED_CLIENT_HELLO => !self.has(Mutation::StripEch),
            ext::COMPRESS_CERTIFICATE => !self.has(Mutation::StripCertCompression),
            _ => true,
        });
        if let Some(seed) = self.permutation {
            shuffle_movable(&mut plan, seed);
        }
        plan
    }

    fn extension_body(&self, kind: u16) -> Vec<u8> {
        let mut body = Vec::new();
        match kind {
            ext::SERVER_NAME => {
                let mut sni = vec![0u8]; // name_type host_name
                put_vec16(&mut sni, self.sni.as_bytes());
                put_vec16(&mut body, &sni);
            }
            ext::SUPPORTED_GROUPS => put_vec16(&mut body, &words(&self.groups())),
            ext::EC_POINT_FORMATS => body.extend_from_slice(&[1, 0]), // uncompressed
            ext::SIGNATURE_ALGORITHMS => {
                let algs = match self.profile {
                    Profile::Firefox => FIREFOX_SIG_ALGS,
                    Profile::Stock => STOCK_SIG_ALGS,
                    Profile::Chrome | Profile::Chrome100 => CHROME_SIG_ALGS,
                };
                put_vec16(&mut body, &words(algs));
            }
            ext::SIGNATURE_ALGORITHMS_CERT => put_vec16(&mut body, &words(STOCK_SIG_ALGS_CERT)),
            ext::DELEGATED_CREDENTIALS => put_vec16(&mut body, &words(FIREFOX_DELEGATED_SIG_ALGS)),
            ext::ALPN => {
                let mut names = Vec::new();
                if self.alpn == Alpn::H2AndHttp11 {
                    put_vec8(&mut names, b"h2");
                }
                put_vec8(&mut names, b"http/1.1");
                put_vec16(&mut body, &names);
            }
            ext::SUPPORTED_VERSIONS => {
                let mut versions = Vec::new();
                if self.profile.greased() {
                    versions.push(GREASE_A);
                }
                versions.extend_from_slice(&[0x0304, 0x0303]);
                put_vec8(&mut body, &words(&versions));
            }
            ext::PSK_KEY_EXCHANGE_MODES => body.extend_from_slice(&[1, 1]), // psk_dhe_ke
            ext::COMPRESS_CERTIFICATE => body.extend_from_slice(&[2, 0, 2]), // brotli
            ext::STATUS_REQUEST => body.extend_from_slice(&[1, 0, 0, 0, 0]), // ocsp
            ext::RECORD_SIZE_LIMIT => body.extend_from_slice(&0x4001u16.to_be_bytes()),
            ext::KEY_SHARE => {
                let mut entries = Vec::new();
                for (group, len) in self.key_shares() {
                    entries.extend_from_slice(&group.to_be_bytes());
                    // GREASE share is a single zero byte; others are constant filler.
                    let fill = if group == GREASE_A { 0 } else { 0x42 };
                    put_vec16(&mut entries, &vec![fill; len]);
                }
                put_vec16(&mut body, &entries);
            }
            ext::APPLICATION_SETTINGS | ext::APPLICATION_SETTINGS_NEW => {
                let mut names = Vec::new();
                put_vec8(&mut names, b"h2");
                put_vec16(&mut body, &names);
            }
            ext::RENEGOTIATION_INFO => body.push(0), // empty renegotiated_connection
            ext::ENCRYPTED_CLIENT_HELLO => {
                // ECH GREASE outer: type, HPKE suite (HKDF-SHA256, AES-128-GCM),
                // config_id, 32-byte enc, payload of the first utls candidate + 16.
                body.push(0); // outer
                body.extend_from_slice(&0x0001u16.to_be_bytes()); // kdf
                body.extend_from_slice(&0x0001u16.to_be_bytes()); // aead
                body.push(0x42); // config_id
                put_vec16(&mut body, &[0x42; 32]);
                let payload = if self.profile == Profile::Firefox {
                    223 + 16
                } else {
                    128 + 16
                };
                put_vec16(&mut body, &vec![0x42; payload]);
            }
            GREASE_B => body.push(0), // utls trailing GREASE carries one zero byte
            // leading GREASE, SCT, extended_master_secret, session_ticket: empty
            _ => {}
        }
        body
    }

    /// Encode the raw ClientHello handshake message (type 1).
    pub fn handshake(&self) -> Vec<u8> {
        let mut head = Vec::new();
        head.extend_from_slice(&0x0303u16.to_be_bytes()); // legacy_version
        head.extend_from_slice(&[0x11u8; 32]); // fixed random (not fingerprinted)
        put_vec8(&mut head, &[0x22u8; 32]); // session id
        let mut ciphers = Vec::new();
        if self.profile.greased() {
            ciphers.push(GREASE_A);
        }
        ciphers.extend_from_slice(match self.profile {
            Profile::Chrome | Profile::Chrome100 => CHROME_CIPHERS,
            Profile::Firefox => FIREFOX_CIPHERS,
            Profile::Stock => STOCK_CIPHERS,
        });
        put_vec16(&mut head, &words(&ciphers));
        put_vec8(&mut head, &[0x00]); // compression: null only

        let plan = self.extension_plan();
        let mut exts = Vec::new();
        for &kind in &plan {
            if kind != ext::PADDING {
                write_extension(&mut exts, kind, &self.extension_body(kind));
            }
        }
        if plan.contains(&ext::PADDING) {
            // utls BoringPaddingStyle over the unpadded handshake length
            // (4-byte handshake header + body + 2-byte extensions length).
            let unpadded = 4 + head.len() + 2 + exts.len();
            if let Some(len) = boring_padding(unpadded) {
                write_extension(&mut exts, ext::PADDING, &vec![0u8; len]);
            }
        }
        let mut body = head;
        put_vec16(&mut body, &exts);

        let mut msg = vec![1u8]; // handshake type: ClientHello
        put_vec24(&mut msg, &body);
        msg
    }

    /// Wrap the handshake in a single TLS 1.0 record (content type 22).
    pub fn record(&self) -> Vec<u8> {
        let handshake = self.handshake();
        let mut record = vec![22u8, 3, 1];
        let len = u16::try_from(handshake.len()).expect("ClientHello under 64 KiB");
        record.extend_from_slice(&len.to_be_bytes());
        record.extend_from_slice(&handshake);
        record
    }
}

/// utls `BoringPaddingStyle`: pad hellos whose unpadded length is in
/// (255, 512) up to 512 bytes, accounting for the 4-byte extension header.
fn boring_padding(unpadded: usize) -> Option<usize> {
    if unpadded > 0xff && unpadded < 0x200 {
        let len = 0x200 - unpadded;
        Some(if len >= 5 { len - 4 } else { 1 })
    } else {
        None
    }
}

/// Deterministic Fisher-Yates over every position that is neither GREASE nor
/// padding (those stay fixed, as in utls `ShuffleChromeTLSExtensions`).
fn shuffle_movable(plan: &mut [u16], seed: u64) {
    let movable: Vec<usize> = (0..plan.len())
        .filter(|&i| !is_grease(plan[i]) && plan[i] != ext::PADDING)
        .collect();
    let mut state = seed;
    let mut next = || {
        // splitmix64
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    for i in (1..movable.len()).rev() {
        let j = usize::try_from(next() % (i as u64 + 1)).expect("index fits usize");
        plan.swap(movable[i], movable[j]);
    }
}

fn is_grease(value: u16) -> bool {
    value & 0x0f0f == 0x0a0a && value >> 8 == value & 0xff
}

fn write_extension(out: &mut Vec<u8>, kind: u16, body: &[u8]) {
    out.extend_from_slice(&kind.to_be_bytes());
    put_vec16(out, body);
}
fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}
fn put_vec8(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(u8::try_from(bytes.len()).expect("vec8 length"));
    out.extend_from_slice(bytes);
}
fn put_vec16(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(
        &u16::try_from(bytes.len())
            .expect("vec16 length")
            .to_be_bytes(),
    );
    out.extend_from_slice(bytes);
}
fn put_vec24(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("vec24 length");
    out.extend_from_slice(&len.to_be_bytes()[1..]);
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_runtime::capture::tls::ClientHelloCapture;
    use humanymous_runtime::network::tls::{
        ClientHello, engine_from_client_hello, ja3, ja4, parse_client_hello,
    };

    fn parsed(builder: &ClientHelloBuilder) -> ClientHello {
        parse_client_hello(&builder.handshake()).expect("builder emits a parseable ClientHello")
    }

    fn captured(builder: &ClientHelloBuilder) -> ClientHello {
        let mut capture = ClientHelloCapture::default();
        capture
            .feed(&builder.record())
            .expect("record framing accepted");
        assert!(capture.is_complete());
        capture.finish().expect("capture yields a ClientHello")
    }

    #[test]
    fn chrome_parrot_is_accepted_by_the_runtime_capture_and_reads_as_chrome() {
        let builder = ClientHelloBuilder::new(Profile::Chrome);
        // The runtime's own strict capture must accept the exact record bytes.
        let captured = captured(&builder);
        // Capture and standalone parse agree on the observed fingerprint.
        assert_eq!(captured, parsed(&builder));
        assert_eq!(engine_from_client_hello(Some(&captured)), "chrome");
        assert!(captured.sni_present);
        assert_eq!(captured.alpn, vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    }

    #[test]
    fn chrome_extension_and_group_order_is_preserved_on_the_wire() {
        let hello = parsed(&ClientHelloBuilder::new(Profile::Chrome));
        // GREASE first, server_name second, a trailing GREASE last.
        assert_eq!(hello.extensions[0], GREASE_A);
        assert_eq!(hello.extensions[1], ext::SERVER_NAME);
        assert_eq!(*hello.extensions.last().unwrap(), GREASE_B);
        assert!(hello.extensions.contains(&ext::APPLICATION_SETTINGS_NEW));
        assert!(hello.extensions.contains(&ext::ENCRYPTED_CLIENT_HELLO));
        assert!(!hello.extensions.contains(&ext::PADDING));
        // PQ hybrid group is offered right after the leading GREASE group.
        assert_eq!(
            hello.curves,
            vec![GREASE_A, GROUP_X25519_MLKEM768, 0x1d, 0x17, 0x18]
        );
    }

    #[test]
    fn each_mutation_drops_exactly_its_target_and_nothing_else() {
        let base = parsed(&ClientHelloBuilder::new(Profile::Chrome));

        let alps = parsed(&ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::StripAlps));
        assert!(!alps.extensions.contains(&ext::APPLICATION_SETTINGS_NEW));
        assert_eq!(alps.extensions.len(), base.extensions.len() - 1);

        let ech = parsed(&ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::StripEch));
        assert!(!ech.extensions.contains(&ext::ENCRYPTED_CLIENT_HELLO));
        assert_eq!(ech.extensions.len(), base.extensions.len() - 1);

        let cc = parsed(
            &ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::StripCertCompression),
        );
        assert!(!cc.extensions.contains(&ext::COMPRESS_CERTIFICATE));
        assert_eq!(cc.extensions.len(), base.extensions.len() - 1);
        // Mutations never touch the groups or ciphers.
        for hello in [&alps, &ech, &cc] {
            assert_eq!(hello.curves, base.curves);
            assert_eq!(hello.cipher_suites, base.cipher_suites);
        }
    }

    #[test]
    fn chrome100_is_the_static_pre_pq_pre_ech_preset_with_boring_padding() {
        let builder = ClientHelloBuilder::new(Profile::Chrome100).force_http11();
        let hello = captured(&builder);
        assert_eq!(engine_from_client_hello(Some(&hello)), "chrome");
        assert!(!hello.curves.contains(&GROUP_X25519_MLKEM768));
        assert!(!hello.extensions.contains(&ext::ENCRYPTED_CLIENT_HELLO));
        assert!(hello.extensions.contains(&ext::APPLICATION_SETTINGS));
        assert_eq!(*hello.extensions.last().unwrap(), ext::PADDING);
        assert_eq!(hello.alpn, vec![b"http/1.1".to_vec()]);
        // BoringPaddingStyle pads this short hello to exactly 512 bytes, as the
        // utls HelloChrome_100 capture with the same SNI does.
        assert_eq!(builder.handshake().len(), 512);
        // permute() is a no-op for a fixed-order preset.
        assert_eq!(
            ClientHelloBuilder::new(Profile::Chrome100)
                .permute(7)
                .extension_plan(),
            ClientHelloBuilder::new(Profile::Chrome100).extension_plan()
        );
    }

    #[test]
    fn firefox_models_the_utls_firefox_120_preset() {
        let hello = captured(&ClientHelloBuilder::new(Profile::Firefox));
        assert!(hello.cipher_suites.iter().all(|&c| !is_grease(c)));
        assert_eq!(hello.cipher_suites.len(), 17);
        assert_eq!(hello.curves, vec![0x1d, 0x17, 0x18, 0x19, 0x0100, 0x0101]);
        assert!(hello.extensions.contains(&ext::RECORD_SIZE_LIMIT));
        assert!(hello.extensions.contains(&ext::DELEGATED_CREDENTIALS));
        assert!(hello.extensions.contains(&ext::ENCRYPTED_CLIENT_HELLO));
        assert!(!hello.extensions.contains(&ext::COMPRESS_CERTIFICATE));
        assert_eq!(hello.signature_algorithms, FIREFOX_SIG_ALGS);
        // The frozen structural heuristic needs GREASE before it reads ext 28
        // as Firefox; utls HelloFirefox_120 sends none, so it reads "openssl".
        assert_eq!(engine_from_client_hello(Some(&hello)), "openssl");
    }

    #[test]
    fn stock_stack_reads_as_a_non_browser_go_fingerprint() {
        let hello = captured(&ClientHelloBuilder::new(Profile::Stock));
        // No GREASE, <=16 ciphers, no compress_certificate -> "go".
        assert_eq!(engine_from_client_hello(Some(&hello)), "go");
        assert!(!hello.extensions.contains(&ext::COMPRESS_CERTIFICATE));
        // Go doStock offers only http/1.1.
        assert_eq!(hello.alpn, vec![b"http/1.1".to_vec()]);
        assert_eq!(hello.curves[0], GROUP_X25519_MLKEM768);
    }

    #[test]
    fn chrome_permutation_changes_ja3_order_but_not_ja4_and_keeps_grease_fixed() {
        let base = ClientHelloBuilder::new(Profile::Chrome);
        let a = parsed(&ClientHelloBuilder::new(Profile::Chrome).permute(1));
        let b = parsed(&ClientHelloBuilder::new(Profile::Chrome).permute(2));
        assert_ne!(a.extensions, b.extensions);
        assert_ne!(ja3(&a).0, ja3(&b).0);
        assert_eq!(ja4(&a).0, ja4(&b).0);
        assert_eq!(ja4(&a).0, ja4(&parsed(&base)).0);
        for hello in [&a, &b] {
            assert_eq!(hello.extensions[0], GREASE_A);
            assert_eq!(*hello.extensions.last().unwrap(), GREASE_B);
        }
        // Deterministic for a given seed.
        assert_eq!(
            ClientHelloBuilder::new(Profile::Chrome)
                .permute(1)
                .extension_plan(),
            a.extensions
        );
    }

    #[test]
    fn each_profile_has_a_distinct_ja3_and_ja4() {
        let profiles = [
            Profile::Chrome,
            Profile::Chrome100,
            Profile::Firefox,
            Profile::Stock,
        ];
        let prints: Vec<_> = profiles
            .iter()
            .map(|&p| {
                let hello = parsed(&ClientHelloBuilder::new(p).force_http11());
                (ja3(&hello).0, ja4(&hello).0)
            })
            .collect();
        for i in 0..prints.len() {
            for j in i + 1..prints.len() {
                assert_ne!(
                    prints[i].0, prints[j].0,
                    "{:?}/{:?}",
                    profiles[i], profiles[j]
                );
                assert_ne!(
                    prints[i].1, prints[j].1,
                    "{:?}/{:?}",
                    profiles[i], profiles[j]
                );
            }
        }
    }

    #[test]
    fn static_presets_match_the_shape_captured_from_the_oracle_toolchain() {
        // Extension type sequences and handshake lengths from utls v1.8.2 /
        // Go 1.25.3 ClientHellos for SNI owned.test, captured in a scratch Go
        // program (not shipped). Chrome_100, Firefox and Go stock match those
        // captures exactly. Chrome_Auto (HelloChrome_133) shuffles its
        // extensions on every build and picks a random ECH payload length, so
        // its two rows assert utls's pre-shuffle declaration order and the
        // handshake length for the first ECH candidate; they are not one
        // observed capture. GREASE positions are shown as GREASE_A/GREASE_B.
        let cases: [(ClientHelloBuilder, &[u16], usize); 5] = [
            (
                ClientHelloBuilder::new(Profile::Chrome100).force_http11(),
                &[
                    GREASE_A, 0, 23, 0xff01, 10, 11, 35, 16, 5, 13, 18, 51, 45, 43, 27, 17513,
                    GREASE_B, 21,
                ],
                512,
            ),
            (
                ClientHelloBuilder::new(Profile::Chrome).force_http11(),
                &[
                    GREASE_A, 0, 23, 0xff01, 10, 11, 35, 16, 5, 13, 18, 51, 45, 43, 27, 17613,
                    0xfe0d, GREASE_B,
                ],
                1712,
            ),
            (
                ClientHelloBuilder::new(Profile::Chrome),
                &[
                    GREASE_A, 0, 23, 0xff01, 10, 11, 35, 16, 5, 13, 18, 51, 45, 43, 27, 17613,
                    0xfe0d, GREASE_B,
                ],
                1715,
            ),
            (
                ClientHelloBuilder::new(Profile::Firefox).force_http11(),
                &[
                    0, 23, 0xff01, 10, 11, 35, 16, 5, 34, 51, 43, 13, 45, 28, 0xfe0d,
                ],
                650,
            ),
            (
                ClientHelloBuilder::new(Profile::Stock),
                &[0, 11, 0xff01, 23, 18, 5, 10, 13, 50, 16, 43, 51],
                1508,
            ),
        ];
        for (builder, extensions, length) in cases {
            assert_eq!(parsed(&builder).extensions, extensions);
            assert_eq!(builder.handshake().len(), length, "{:?}", builder.profile);
        }
    }

    #[test]
    fn ja3_is_stable_and_excludes_grease() {
        let (hash, text) = ja3(&parsed(&ClientHelloBuilder::new(Profile::Chrome)));
        assert_eq!(hash.len(), 32); // md5 hex
        assert_eq!(
            hash,
            ja3(&parsed(&ClientHelloBuilder::new(Profile::Chrome))).0
        );
        // GREASE values are filtered out of the JA3 cipher/extension lists.
        assert!(!text.contains("2570")); // 0x0a0a
        assert!(!text.contains("6682")); // 0x1a1a
    }
}
