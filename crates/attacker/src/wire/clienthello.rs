//! Byte-exact TLS ClientHello construction for the owned-lab parrot clients.
//!
//! Defense-only. This reproduces the JA3/JA4-relevant wire shape the Go uTLS
//! parrot clients (`cmd/tlsparrot`, `cmd/redteam`) presented so the Rust port
//! can emit the same ClientHello against the project's own deployment. It only
//! *constructs* bytes; see the crate-level gap notes on live handshakes. The
//! field ordering and extension set are the fingerprint under test, so every
//! builder output is validated through the runtime's own ClientHello observer.
//!
//! No arbitrary-target capability exists here: the bytes carry only a loopback
//! SNI and are exercised against the owned runtime's capture path in tests.

/// GREASE is an even grease-coded value; a second distinct grease type is used
/// for the trailing extension so the capture's duplicate-type guard stays quiet.
const GREASE_A: u16 = 0x0a0a;
const GREASE_B: u16 = 0x1a1a;

/// Extension codepoints (wire-order significant for the fingerprint).
mod ext {
    pub const SERVER_NAME: u16 = 0;
    pub const STATUS_REQUEST: u16 = 5;
    pub const SUPPORTED_GROUPS: u16 = 10;
    pub const EC_POINT_FORMATS: u16 = 11;
    pub const SIGNATURE_ALGORITHMS: u16 = 13;
    pub const ALPN: u16 = 16;
    pub const SCT: u16 = 18;
    pub const PADDING: u16 = 21;
    pub const EXTENDED_MASTER_SECRET: u16 = 23;
    pub const COMPRESS_CERTIFICATE: u16 = 27;
    pub const SESSION_TICKET: u16 = 35;
    pub const SUPPORTED_VERSIONS: u16 = 43;
    pub const PSK_KEY_EXCHANGE_MODES: u16 = 45;
    pub const KEY_SHARE: u16 = 51;
    pub const APPLICATION_SETTINGS: u16 = 17513;
    pub const ENCRYPTED_CLIENT_HELLO: u16 = 0xfe0d;
    pub const RENEGOTIATION_INFO: u16 = 0xff01;
}

/// X25519MLKEM768 hybrid post-quantum group (Chrome 131+ default).
const GROUP_X25519_MLKEM768: u16 = 0x11ec;

/// Chrome-family cipher suites in wire order (TLS 1.3 then 1.2).
const CHROME_CIPHERS: &[u16] = &[
    0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc013, 0xc014, 0x009c,
    0x009d, 0x002f, 0x0035,
];
/// A plain non-browser stack: no TLS 1.3 cipher churn, <=16 suites, no GREASE.
const STOCK_CIPHERS: &[u16] = &[
    0xc02f, 0xc02b, 0xc030, 0xc02c, 0xc013, 0xc009, 0xc014, 0xc00a, 0x009c, 0x009d, 0x002f, 0x0035,
];
const SIG_ALGS: &[u16] = &[
    0x0403, 0x0804, 0x0401, 0x0503, 0x0805, 0x0501, 0x0806, 0x0601,
];

/// Which parrot profile and which single-extension mutation to emit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// A coherent Chrome ClientHello (GREASE, PQ group, ALPS, ECH, cert-comp).
    Chrome,
    /// A genuine non-browser TLS stack (no GREASE, no cert-compression).
    Stock,
}

/// A single-extension deviation from the coherent Chrome parrot, isolating the
/// residual tell each Go attack targeted. `None` keeps the full coherent hello.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// Drop the ALPS application_settings extension (`alps-absent`).
    StripAlps,
    /// Drop the Encrypted Client Hello GREASE extension (`ech-absent`).
    StripEch,
    /// Drop compress_certificate, ext 27 (`cert-compression-absent`).
    StripCertCompression,
    /// Omit the X25519MLKEM768 PQ key share group (`pq-absent`).
    PreQuantum,
}

/// A ClientHello builder. Random/session bytes are fixed for reproducibility;
/// they are not fingerprint-bearing fields.
pub struct ClientHelloBuilder {
    profile: Profile,
    mutations: Vec<Mutation>,
    sni: String,
}

impl ClientHelloBuilder {
    /// Start from a parrot profile targeting the loopback owned runtime.
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            mutations: Vec::new(),
            sni: "owned.test".into(),
        }
    }

    /// Apply a single-extension deviation (the residual tell under test).
    pub fn mutate(mut self, mutation: Mutation) -> Self {
        self.mutations.push(mutation);
        self
    }

    fn has(&self, m: Mutation) -> bool {
        self.mutations.contains(&m)
    }

    fn groups(&self) -> Vec<u16> {
        let mut groups = vec![GREASE_A];
        if self.profile == Profile::Chrome && !self.has(Mutation::PreQuantum) {
            groups.push(GROUP_X25519_MLKEM768);
        }
        groups.extend_from_slice(&[0x001d, 0x0017, 0x0018]);
        groups
    }

    /// The extension types, in wire order, for the selected profile/mutations.
    fn extension_plan(&self) -> Vec<u16> {
        if self.profile == Profile::Stock {
            // A minimal non-browser order: no GREASE, no cert-compression/ALPS/ECH.
            return vec![
                ext::SERVER_NAME,
                ext::SUPPORTED_GROUPS,
                ext::EC_POINT_FORMATS,
                ext::SIGNATURE_ALGORITHMS,
                ext::ALPN,
                ext::SUPPORTED_VERSIONS,
                ext::KEY_SHARE,
            ];
        }
        let mut plan = vec![
            GREASE_A,
            ext::SERVER_NAME,
            ext::EXTENDED_MASTER_SECRET,
            ext::RENEGOTIATION_INFO,
            ext::SUPPORTED_GROUPS,
            ext::EC_POINT_FORMATS,
            ext::SESSION_TICKET,
            ext::ALPN,
            ext::STATUS_REQUEST,
            ext::SIGNATURE_ALGORITHMS,
            ext::SCT,
            ext::KEY_SHARE,
            ext::PSK_KEY_EXCHANGE_MODES,
            ext::SUPPORTED_VERSIONS,
            ext::COMPRESS_CERTIFICATE,
            ext::APPLICATION_SETTINGS,
            ext::ENCRYPTED_CLIENT_HELLO,
            GREASE_B,
            ext::PADDING,
        ];
        plan.retain(|&e| match e {
            ext::APPLICATION_SETTINGS => !self.has(Mutation::StripAlps),
            ext::ENCRYPTED_CLIENT_HELLO => !self.has(Mutation::StripEch),
            ext::COMPRESS_CERTIFICATE => !self.has(Mutation::StripCertCompression),
            _ => true,
        });
        plan
    }

    fn write_extension(&self, out: &mut Vec<u8>, kind: u16) {
        let body: Vec<u8> = match kind {
            ext::SERVER_NAME => {
                let mut sni = vec![0u8]; // name_type host_name
                put_vec16(&mut sni, self.sni.as_bytes());
                let mut list = Vec::new();
                put_vec16(&mut list, &sni);
                list
            }
            ext::SUPPORTED_GROUPS => {
                let mut groups = Vec::new();
                for g in self.groups() {
                    groups.extend_from_slice(&g.to_be_bytes());
                }
                let mut body = Vec::new();
                put_vec16(&mut body, &groups);
                body
            }
            ext::EC_POINT_FORMATS => vec![1, 0], // one format: uncompressed
            ext::SIGNATURE_ALGORITHMS => {
                let mut algs = Vec::new();
                for a in SIG_ALGS {
                    algs.extend_from_slice(&a.to_be_bytes());
                }
                let mut body = Vec::new();
                put_vec16(&mut body, &algs);
                body
            }
            ext::ALPN => {
                let mut names = Vec::new();
                put_vec8(&mut names, b"h2");
                put_vec8(&mut names, b"http/1.1");
                let mut body = Vec::new();
                put_vec16(&mut body, &names);
                body
            }
            ext::SUPPORTED_VERSIONS => {
                let mut vers = Vec::new();
                for v in [GREASE_A, 0x0304u16, 0x0303u16] {
                    vers.extend_from_slice(&v.to_be_bytes());
                }
                let mut body = Vec::new();
                put_vec8(&mut body, &vers);
                body
            }
            ext::PSK_KEY_EXCHANGE_MODES => vec![1, 1], // psk_dhe_ke
            ext::COMPRESS_CERTIFICATE => vec![2, 0, 2], // brotli
            ext::STATUS_REQUEST => vec![1, 0, 0, 0, 0], // ocsp, empty responder/exts
            ext::KEY_SHARE => {
                // One x25519 share; content is ignored by the observation parser.
                let mut entry = Vec::new();
                entry.extend_from_slice(&0x001du16.to_be_bytes());
                put_vec16(&mut entry, &[0x42u8; 32]);
                let mut body = Vec::new();
                put_vec16(&mut body, &entry);
                body
            }
            ext::APPLICATION_SETTINGS => {
                let mut names = Vec::new();
                put_vec8(&mut names, b"h2");
                let mut body = Vec::new();
                put_vec16(&mut body, &names);
                body
            }
            ext::RENEGOTIATION_INFO => vec![0], // empty renegotiated_connection
            ext::ENCRYPTED_CLIENT_HELLO | ext::PADDING => vec![0u8; 16],
            _ => Vec::new(), // GREASE, SCT, extended_master_secret, session_ticket: empty
        };
        out.extend_from_slice(&kind.to_be_bytes());
        put_vec16(out, &body);
    }

    fn ciphers(&self) -> Vec<u16> {
        match self.profile {
            Profile::Chrome => {
                let mut c = vec![GREASE_A];
                c.extend_from_slice(CHROME_CIPHERS);
                c
            }
            Profile::Stock => STOCK_CIPHERS.to_vec(),
        }
    }

    /// Encode the raw ClientHello handshake message (type 1).
    pub fn handshake(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0x0303u16.to_be_bytes()); // legacy_version
        body.extend_from_slice(&[0x11u8; 32]); // fixed random (not fingerprinted)
        put_vec8(&mut body, &[0x22u8; 32]); // session id
        let mut ciphers = Vec::new();
        for c in self.ciphers() {
            ciphers.extend_from_slice(&c.to_be_bytes());
        }
        put_vec16(&mut body, &ciphers);
        put_vec8(&mut body, &[0x00]); // compression: null only
        let mut exts = Vec::new();
        for kind in self.extension_plan() {
            self.write_extension(&mut exts, kind);
        }
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
        ClientHello, engine_from_client_hello, ja3, parse_client_hello,
    };

    fn parsed(builder: &ClientHelloBuilder) -> ClientHello {
        parse_client_hello(&builder.handshake()).expect("builder emits a parseable ClientHello")
    }

    #[test]
    fn chrome_parrot_is_accepted_by_the_runtime_capture_and_reads_as_chrome() {
        let builder = ClientHelloBuilder::new(Profile::Chrome);
        // The runtime's own strict capture must accept the exact record bytes.
        let mut capture = ClientHelloCapture::default();
        capture
            .feed(&builder.record())
            .expect("record framing accepted");
        assert!(capture.is_complete());
        let captured = capture.finish().expect("capture yields a ClientHello");
        // Capture and standalone parse agree on the observed fingerprint.
        assert_eq!(captured, parsed(&builder));
        assert_eq!(engine_from_client_hello(Some(&captured)), "chrome");
        assert!(captured.sni_present);
        assert_eq!(captured.alpn, vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    }

    #[test]
    fn chrome_extension_and_group_order_is_preserved_on_the_wire() {
        let hello = parsed(&ClientHelloBuilder::new(Profile::Chrome));
        // GREASE first, server_name second, ECH then a trailing GREASE near the end.
        assert_eq!(hello.extensions[0], GREASE_A);
        assert_eq!(hello.extensions[1], ext::SERVER_NAME);
        assert!(hello.extensions.contains(&ext::APPLICATION_SETTINGS));
        assert!(hello.extensions.contains(&ext::ENCRYPTED_CLIENT_HELLO));
        // PQ hybrid group is offered right after the leading GREASE group.
        assert_eq!(hello.curves[0], GREASE_A);
        assert_eq!(hello.curves[1], GROUP_X25519_MLKEM768);
    }

    #[test]
    fn each_mutation_drops_exactly_its_target_and_nothing_else() {
        let base = parsed(&ClientHelloBuilder::new(Profile::Chrome));

        let alps = parsed(&ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::StripAlps));
        assert!(!alps.extensions.contains(&ext::APPLICATION_SETTINGS));
        assert_eq!(alps.extensions.len(), base.extensions.len() - 1);

        let ech = parsed(&ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::StripEch));
        assert!(!ech.extensions.contains(&ext::ENCRYPTED_CLIENT_HELLO));

        let cc = parsed(
            &ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::StripCertCompression),
        );
        assert!(!cc.extensions.contains(&ext::COMPRESS_CERTIFICATE));

        let pq = parsed(&ClientHelloBuilder::new(Profile::Chrome).mutate(Mutation::PreQuantum));
        assert!(!pq.curves.contains(&GROUP_X25519_MLKEM768));
        // dropping the PQ group leaves the extension set intact.
        assert_eq!(pq.extensions.len(), base.extensions.len());
    }

    #[test]
    fn stock_stack_reads_as_a_non_browser_go_fingerprint() {
        let builder = ClientHelloBuilder::new(Profile::Stock);
        let mut capture = ClientHelloCapture::default();
        capture
            .feed(&builder.record())
            .expect("stock record accepted");
        let hello = capture.finish().expect("stock ClientHello");
        // No GREASE, <=16 ciphers, no compress_certificate -> "go".
        assert_eq!(engine_from_client_hello(Some(&hello)), "go");
        assert!(!hello.extensions.contains(&ext::COMPRESS_CERTIFICATE));
    }

    #[test]
    fn ja3_is_stable_and_excludes_grease() {
        let (hash, text) = ja3(&parsed(&ClientHelloBuilder::new(Profile::Chrome)));
        assert_eq!(hash.len(), 32); // md5 hex
        // GREASE values are filtered out of the JA3 cipher/extension lists.
        assert!(!text.contains("2570")); // 0x0a0a
        assert!(!text.contains("6682")); // 0x1a1a
    }
}
