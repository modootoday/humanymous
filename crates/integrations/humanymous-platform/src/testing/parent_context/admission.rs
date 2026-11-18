//! Creator-channel envelope `humanymous.dotnet-host-admission/1` (SoT
//! dotnet-protected-host-launch 3.1). One bounded frame: a strict JSON header
//! line followed by the exact context bytes and the exact binding bytes. The
//! codec checks syntax and bounds only; it never establishes that the bytes
//! came from the creator. Only the consumer's handle facts can do that.
use super::{WireError, host::nonce};
use crate::testing::json::decode_strict_json;
use serde::{Deserialize, Serialize};

/// Fixed envelope schema.
pub const SCHEMA: &str = "humanymous.dotnet-host-admission/1";
/// Whole frame bound: 64 KiB context, 16 KiB binding, plus framing.
pub const MAX_ENVELOPE: usize = 96 * 1024;
const HEADER_LIMIT: usize = 4096;
const CONTEXT_LIMIT: usize = 64 * 1024;
const BINDING_LIMIT: usize = 16 * 1024;

/// The single existing custody provider profile (plan 20261107115350).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionProfile {
    /// One provider with a Windows and a Linux backend.
    #[serde(rename = "standard-user-dotnet-prebuilt")]
    StandardUserDotnetPrebuilt,
}

/// Fixed package-matrix mode labels for a derived admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivedMode {
    /// JIT build from project references.
    #[serde(rename = "jit-source")]
    JitSource,
    /// JIT build from the packed adapter.
    #[serde(rename = "jit-packed")]
    JitPacked,
    /// NativeAOT build from project references.
    #[serde(rename = "aot-source")]
    AotSource,
    /// NativeAOT build from the packed adapter.
    #[serde(rename = "aot-packed")]
    AotPacked,
}

/// Narrowing a same-actor controller applies for one mode's apphost. It can
/// only replace the apphost and audit root; the consumer requires both under
/// the ORIGINAL private_work. Actor, nonce and budget are never carried here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derivation {
    /// Fixed mode label.
    pub mode: DerivedMode,
    /// Claimed per-mode apphost under the original private_work.
    pub apphost: String,
    /// Claimed per-mode audit root under the original private_work.
    pub audit_root: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: String,
    profile: AdmissionProfile,
    run_nonce: String,
    derived: Option<Derivation>,
    context_bytes: usize,
    binding_bytes: usize,
}

/// Syntax-checked envelope. Exact byte fields are returned unchanged.
#[derive(Debug, PartialEq, Eq)]
pub struct UnverifiedEnvelope {
    /// Claimed provider profile.
    pub profile: AdmissionProfile,
    /// Claimed original run nonce.
    pub run_nonce: String,
    /// Optional same-actor narrowing.
    pub derived: Option<Derivation>,
    /// Exact context wire bytes.
    pub context: Vec<u8>,
    /// Exact binding wire bytes.
    pub binding: Vec<u8>,
}

fn derivation(value: Option<&Derivation>) -> bool {
    value.is_none_or(|value| {
        [&value.apphost, &value.audit_root]
            .into_iter()
            .all(|text| super::bounded(text))
    })
}

/// Encode one frame: compact header, LF, context bytes, binding bytes.
pub fn encode_envelope(envelope: &UnverifiedEnvelope) -> Result<Vec<u8>, WireError> {
    if !nonce(&envelope.run_nonce)
        || !derivation(envelope.derived.as_ref())
        || envelope.context.is_empty()
        || envelope.binding.is_empty()
        || envelope.context.len() > CONTEXT_LIMIT
        || envelope.binding.len() > BINDING_LIMIT
    {
        return Err(WireError);
    }
    let header = Header {
        schema: SCHEMA.into(),
        profile: envelope.profile,
        run_nonce: envelope.run_nonce.clone(),
        derived: envelope.derived.clone(),
        context_bytes: envelope.context.len(),
        binding_bytes: envelope.binding.len(),
    };
    let mut bytes = serde_json::to_vec(&header).map_err(|_| WireError)?;
    if bytes.len() > HEADER_LIMIT {
        return Err(WireError);
    }
    bytes.push(b'\n');
    bytes.extend_from_slice(&envelope.context);
    bytes.extend_from_slice(&envelope.binding);
    if bytes.len() > MAX_ENVELOPE {
        return Err(WireError);
    }
    Ok(bytes)
}

/// Decode exactly one complete frame. Trailing, missing, duplicate-key or
/// oversize input fails without echoing bytes.
pub fn decode_envelope(bytes: &[u8]) -> Result<UnverifiedEnvelope, WireError> {
    if bytes.len() > MAX_ENVELOPE {
        return Err(WireError);
    }
    let line = bytes
        .iter()
        .take(HEADER_LIMIT + 1)
        .position(|byte| *byte == b'\n')
        .ok_or(WireError)?;
    let value = decode_strict_json(&bytes[..line], HEADER_LIMIT).map_err(|_| WireError)?;
    let header: Header = serde_json::from_value(value).map_err(|_| WireError)?;
    let rest = &bytes[line + 1..];
    if header.schema != SCHEMA
        || !nonce(&header.run_nonce)
        || !derivation(header.derived.as_ref())
        || header.context_bytes == 0
        || header.binding_bytes == 0
        || header.context_bytes > CONTEXT_LIMIT
        || header.binding_bytes > BINDING_LIMIT
        || header.context_bytes.checked_add(header.binding_bytes) != Some(rest.len())
    {
        return Err(WireError);
    }
    let (context, binding) = rest.split_at(header.context_bytes);
    Ok(UnverifiedEnvelope {
        profile: header.profile,
        run_nonce: header.run_nonce,
        derived: header.derived,
        context: context.to_vec(),
        binding: binding.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> UnverifiedEnvelope {
        UnverifiedEnvelope {
            profile: AdmissionProfile::StandardUserDotnetPrebuilt,
            run_nonce: "ab".repeat(16),
            derived: None,
            context: b"{\"context\":1}\n".to_vec(),
            binding: b"{\"binding\":1}\n".to_vec(),
        }
    }

    #[test]
    fn exact_frame_round_trips_and_keeps_bytes() {
        let bytes = encode_envelope(&envelope()).unwrap();
        assert_eq!(decode_envelope(&bytes).unwrap(), envelope());
        let mut derived = envelope();
        derived.derived = Some(Derivation {
            mode: DerivedMode::AotPacked,
            apphost: "/srv/work/aot-packed/Humanymous.HostPilot".into(),
            audit_root: "/srv/work/aot-packed/audit".into(),
        });
        let bytes = encode_envelope(&derived).unwrap();
        assert_eq!(decode_envelope(&bytes).unwrap(), derived);
    }

    #[test]
    fn trailing_truncated_duplicate_and_oversize_frames_fail() {
        let bytes = encode_envelope(&envelope()).unwrap();
        let mut trailing = bytes.clone();
        trailing.push(b'x');
        assert!(decode_envelope(&trailing).is_err());
        assert!(decode_envelope(&bytes[..bytes.len() - 1]).is_err());
        let doubled = [bytes.as_slice(), bytes.as_slice()].concat();
        assert!(decode_envelope(&doubled).is_err());
        assert!(decode_envelope(b"").is_err());
        assert!(decode_envelope(&vec![b'{'; MAX_ENVELOPE + 1]).is_err());
        let text = String::from_utf8(bytes.clone()).unwrap();
        let duplicate = text.replacen("{\"schema\"", "{\"schema\":\"x\",\"schema\"", 1);
        assert!(decode_envelope(duplicate.as_bytes()).is_err());
        let unknown = text.replacen("\"profile\"", "\"locator\":\"/tmp/x\",\"profile\"", 1);
        assert!(decode_envelope(unknown.as_bytes()).is_err());
        let profile = text.replace("standard-user-dotnet-prebuilt", "current-user");
        assert!(decode_envelope(profile.as_bytes()).is_err());
    }

    #[test]
    fn field_bounds_refuse_before_framing() {
        let mut big = envelope();
        big.context = vec![b' '; CONTEXT_LIMIT + 1];
        assert!(encode_envelope(&big).is_err());
        let mut big = envelope();
        big.binding = vec![b' '; BINDING_LIMIT + 1];
        assert!(encode_envelope(&big).is_err());
        let mut zero = envelope();
        zero.run_nonce = "0".repeat(32);
        assert!(encode_envelope(&zero).is_err());
        let mut empty = envelope();
        empty.binding.clear();
        assert!(encode_envelope(&empty).is_err());
        let mut control = envelope();
        control.derived = Some(Derivation {
            mode: DerivedMode::JitSource,
            apphost: "/srv/work/a\nb".into(),
            audit_root: "/srv/work/audit".into(),
        });
        assert!(encode_envelope(&control).is_err());
        let maximum = UnverifiedEnvelope {
            context: vec![b' '; CONTEXT_LIMIT],
            binding: vec![b' '; BINDING_LIMIT],
            ..envelope()
        };
        let bytes = encode_envelope(&maximum).unwrap();
        assert!(bytes.len() <= MAX_ENVELOPE);
        assert_eq!(decode_envelope(&bytes).unwrap(), maximum);
    }
}
