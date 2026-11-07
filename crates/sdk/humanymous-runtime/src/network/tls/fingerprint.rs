use super::{ClientHello, is_grease};
use sha2::{Digest, Sha256};

fn filtered(values: &[u16]) -> Vec<u16> {
    values
        .iter()
        .copied()
        .filter(|&value| !is_grease(value))
        .collect()
}
fn hex_list(values: &[u16]) -> String {
    values
        .iter()
        .map(|value| format!("{value:04x}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))[..12].into()
}

/// Legacy JA3 compatibility fingerprint and unhashed observation. MD5 is used
/// only because the JA3 wire format mandates it; this is not an authentication
/// hash, integrity proof or security boundary.
pub fn ja3(hello: &ClientHello) -> (String, String) {
    let version = if hello.supported_versions.is_empty() {
        hello.legacy_version
    } else {
        filtered(&hello.supported_versions)
            .into_iter()
            .max()
            .unwrap_or_default()
    };
    let list = |values: &[u16]| {
        filtered(values)
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join("-")
    };
    let text = format!(
        "{version},{},{},{},{}",
        list(&hello.cipher_suites),
        list(&hello.extensions),
        list(&hello.curves),
        list(&hello.point_formats)
    );
    let hash = hex::encode(<md5::Md5 as md5::Digest>::digest(text.as_bytes()));
    (hash, text)
}

/// Resolve the frozen structural engine heuristic, not an identity guarantee.
/// None represents an unavailable capture and returns unknown.
pub fn engine_from_client_hello(hello: Option<&ClientHello>) -> &'static str {
    let Some(hello) = hello else {
        return "unknown";
    };
    let greased = hello
        .cipher_suites
        .iter()
        .chain(&hello.extensions)
        .any(|&v| is_grease(v));
    let has = |extension| hello.extensions.contains(&extension);
    if !greased {
        return if filtered(&hello.cipher_suites).len() <= 16 && !has(27) {
            "go"
        } else {
            "openssl"
        };
    }
    if has(28) {
        "firefox"
    } else if has(27) || has(0xfe0d) || has(0x4469) {
        "chrome"
    } else {
        "safari"
    }
}

/// Hash non-GREASE extension order for the session traffic guard. Empty capture
/// and GREASE-only lists produce no observation.
pub fn extension_order_hash(hello: Option<&ClientHello>) -> String {
    let Some(hello) = hello else {
        return String::new();
    };
    let extensions = filtered(&hello.extensions);
    if extensions.is_empty() {
        return String::new();
    }
    digest(
        &extensions
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join("-"),
    )
}

/// Compute hashed JA4 and raw JA4_r from the captured order-preserving hello.
/// Signature algorithms retain their wire order; cipher/extension lists sort.
pub fn ja4(hello: &ClientHello) -> (String, String) {
    let version = if hello.supported_versions.is_empty() {
        hello.legacy_version
    } else {
        filtered(&hello.supported_versions)
            .into_iter()
            .max()
            .unwrap_or_default()
    };
    let version = match version {
        0x304 => "13",
        0x303 => "12",
        0x302 => "11",
        0x301 => "10",
        _ => "00",
    };
    let sni = if hello.sni_present { 'd' } else { 'i' };
    let alpn = hello
        .alpn
        .first()
        .filter(|name| !name.is_empty())
        .map(|name| {
            // Go's string(byte) converts each selected byte to a Unicode scalar.
            format!(
                "{}{}",
                char::from(name[0]),
                char::from(name[name.len() - 1])
            )
        })
        .unwrap_or_else(|| "00".into());
    let mut ciphers = filtered(&hello.cipher_suites);
    let mut extensions = filtered(&hello.extensions);
    let prefix = format!(
        "t{version}{sni}{:02}{:02}{alpn}",
        ciphers.len().min(99),
        extensions.len().min(99)
    );
    ciphers.sort_unstable();
    extensions.retain(|&value| value != 0 && value != 16);
    extensions.sort_unstable();
    let cipher_text = hex_list(&ciphers);
    let extension_text = format!(
        "{}_{}",
        hex_list(&extensions),
        hex_list(&hello.signature_algorithms)
    );
    let cipher_hash = if ciphers.is_empty() {
        "000000000000".into()
    } else {
        digest(&cipher_text)
    };
    let extension_hash = if extensions.is_empty() {
        "000000000000".into()
    } else {
        digest(&extension_text)
    };
    (
        format!("{prefix}_{cipher_hash}_{extension_hash}"),
        format!("{prefix}_{cipher_text}_{extension_text}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permutation_and_all_grease_codes_preserve_ja4_but_signature_order_does_not() {
        let hello = ClientHello {
            legacy_version: 0x303,
            supported_versions: vec![0x304],
            cipher_suites: vec![0x1301, 0x1302],
            extensions: vec![0, 10, 13, 16, 43],
            signature_algorithms: vec![0x403, 0x804],
            alpn: vec![b"h2".to_vec()],
            sni_present: true,
            ..Default::default()
        };
        let expected = ja4(&hello);
        for high in 0..16_u16 {
            let mut changed = hello.clone();
            let grease = ((high << 4) | 10) * 257;
            changed.cipher_suites.reverse();
            changed.extensions.reverse();
            changed.cipher_suites.push(grease);
            changed.extensions.push(grease);
            changed.supported_versions.push(grease);
            assert_eq!(ja4(&changed), expected);
        }
        let mut changed = hello;
        changed.signature_algorithms.reverse();
        assert_ne!(ja4(&changed), expected);
    }

    #[test]
    fn structural_engine_precedence_and_missing_capture_are_explicit() {
        assert_eq!(engine_from_client_hello(None), "unknown");
        assert_eq!(extension_order_hash(None), "");
        let mut hello = ClientHello::default();
        assert_eq!(engine_from_client_hello(Some(&hello)), "go");
        hello.cipher_suites = vec![1; 17];
        assert_eq!(engine_from_client_hello(Some(&hello)), "openssl");
        hello.cipher_suites.push(0x0a0a);
        assert_eq!(engine_from_client_hello(Some(&hello)), "safari");
        hello.extensions = vec![27, 28];
        assert_eq!(engine_from_client_hello(Some(&hello)), "firefox");
        hello.extensions.pop();
        assert_eq!(engine_from_client_hello(Some(&hello)), "chrome");
        let first = extension_order_hash(Some(&hello));
        hello.extensions.push(13);
        assert_ne!(extension_order_hash(Some(&hello)), first);
        hello.extensions = vec![0x0a0a];
        assert_eq!(extension_order_hash(Some(&hello)), "");
    }
}
