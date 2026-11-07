//! Single metadata binding codec. Equality and hashes never authenticate a parent.
use super::{
    UnverifiedBudget, UnverifiedParentContext, UnverifiedProvenance, WireError, bounded, hex,
};
use crate::json::decode_strict_json;
use serde::{Deserialize, Serialize};

const LIMIT: usize = 16 * 1024;
const SCHEMA: &str = "humanymous.parent-context-binding/1";

/// Closed parent-reported venue metadata, NEVER execution authority.
#[derive(Serialize, Deserialize)]
pub enum PreflightEnvironment {
    /// Reported hosted venue; independent native/external preflight is required.
    #[serde(rename = "github-hosted")]
    GithubHosted,
}

/// Unauthenticated protected-carrier claims with fixed declared serialization
/// order. No native identity, artifact, source custody, Ready or deadline is minted.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedBinding {
    /// Must equal humanymous.parent-context-binding/1.
    pub schema: String,
    /// Claimed original run nonce, canonical nonzero lowercase hex32.
    pub run_nonce: String,
    /// Claimed actor SID; separately compare with actual native inspection.
    pub actor_sid: String,
    /// Producer/invocation commit ONLY, not a common compiled-lineage commit.
    pub source_sha: String,
    /// Digest of exact context wire bytes, independently checked by the caller.
    pub context_sha256: String,
    /// Independent verified-manifest commitment, not self-certified by this DTO.
    pub verified_manifest_sha256: String,
    /// Digest of encode_provenance's compact typed bytes, not arbitrary Map JSON.
    pub provenance_sha256: String,
    /// Exact original context budget, including elapsed dispatch offset.
    pub original_budget: UnverifiedBudget,
    /// Venue metadata only; never enables GHA variables or launch authority.
    pub preflight_environment: PreflightEnvironment,
}
impl UnverifiedBinding {
    fn validate(&self, context: &UnverifiedParentContext) -> Result<(), WireError> {
        context.validate()?;
        let (actor, budget) = match context {
            UnverifiedParentContext::Workload(value) => (&value.actor_sid, &value.budget),
            UnverifiedParentContext::CleanupOnly(value) => (&value.actor_sid, &value.budget),
        };
        if self.schema != SCHEMA
            || !hex(&self.run_nonce, 32)
            || self.run_nonce.bytes().all(|byte| byte == b'0')
            || !bounded(&self.actor_sid)
            || self.actor_sid != *actor
            || !hex(&self.source_sha, 40)
            || !hex(&self.context_sha256, 64)
            || !hex(&self.verified_manifest_sha256, 64)
            || !hex(&self.provenance_sha256, 64)
            || self.original_budget != *budget
        {
            return Err(WireError);
        }
        Ok(())
    }
}

/// Encode fixed typed fields in declaration order, compact JSON plus exactly LF.
/// The original budget MUST equal the context; no clock mapping/renewal occurs.
pub fn encode_binding(
    binding: &UnverifiedBinding,
    context: &UnverifiedParentContext,
) -> Result<Vec<u8>, WireError> {
    binding.validate(context)?;
    let mut bytes = serde_json::to_vec(binding).map_err(|_| WireError)?;
    bytes.push(b'\n');
    if bytes.len() > LIMIT {
        return Err(WireError);
    }
    Ok(bytes)
}

/// Strict bounded decode and exact actor/budget claim comparison. This performs
/// NO hash calculation, native inspection, source verification or trust upgrade.
pub fn decode_binding(
    bytes: &[u8],
    context: &UnverifiedParentContext,
) -> Result<UnverifiedBinding, WireError> {
    let value = decode_strict_json(bytes, LIMIT).map_err(|_| WireError)?;
    let binding: UnverifiedBinding = serde_json::from_value(value).map_err(|_| WireError)?;
    binding.validate(context)?;
    Ok(binding)
}

/// Canonical provenance commitment bytes: compact typed JSON, NO trailing LF.
/// Order is managed/native/harness; each nested record follows its declared
/// source/required-source-or-lock/receipt field order. Never serialize a Map.
pub fn encode_provenance(value: &UnverifiedProvenance) -> Result<Vec<u8>, WireError> {
    value.validate_syntax()?;
    serde_json::to_vec(value).map_err(|_| WireError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn context() -> UnverifiedParentContext {
        let value = json!({"schema":"humanymous.dotnet-prebuilt-launch/1","apphost":"C:/app/Host.exe","app_root":"C:/app","system_root":"C:/Windows","runtime_root":"C:/runtime","private_work":"C:/work","audit_root":"C:/work/audit","handoff_root":"C:/handoff","actor_sid":"S-1-5-21-1-2-3-1001","provenance":{"managed":{"source_sha":"a".repeat(40),"required_source_sha":"b".repeat(40),"receipt_sha256":"c".repeat(64)},"native":{"source_sha":"d".repeat(40),"cargo_lock_sha256":"e".repeat(64),"receipt_sha256":"f".repeat(64)},"harness":{"source_sha":"1".repeat(40),"cargo_lock_sha256":"2".repeat(64),"receipt_sha256":"3".repeat(64)}},"budget":{"issued_utc_ms":1000,"run_expires_utc_ms":181000,"cleanup_expires_utc_ms":241000,"parent_elapsed_ms_at_dispatch":100}});
        super::super::decode(&serde_json::to_vec(&value).unwrap()).unwrap()
    }
    fn binding() -> UnverifiedBinding {
        UnverifiedBinding {
            schema: SCHEMA.into(),
            run_nonce: "a".repeat(32),
            actor_sid: "S-1-5-21-1-2-3-1001".into(),
            source_sha: "9".repeat(40),
            context_sha256: "4".repeat(64),
            verified_manifest_sha256: "5".repeat(64),
            provenance_sha256: "6".repeat(64),
            original_budget: UnverifiedBudget {
                issued_utc_ms: 1000,
                run_expires_utc_ms: 181000,
                cleanup_expires_utc_ms: 241000,
                parent_elapsed_ms_at_dispatch: 100,
            },
            preflight_environment: PreflightEnvironment::GithubHosted,
        }
    }
    #[test]
    fn golden_binding_has_exact_field_order_and_distinct_invocation_source() {
        let context = context();
        let bytes = encode_binding(&binding(), &context).unwrap();
        let expected = format!(
            "{{\"schema\":\"humanymous.parent-context-binding/1\",\"run_nonce\":\"{}\",\"actor_sid\":\"S-1-5-21-1-2-3-1001\",\"source_sha\":\"{}\",\"context_sha256\":\"{}\",\"verified_manifest_sha256\":\"{}\",\"provenance_sha256\":\"{}\",\"original_budget\":{{\"issued_utc_ms\":1000,\"run_expires_utc_ms\":181000,\"cleanup_expires_utc_ms\":241000,\"parent_elapsed_ms_at_dispatch\":100}},\"preflight_environment\":\"github-hosted\"}}\n",
            "a".repeat(32),
            "9".repeat(40),
            "4".repeat(64),
            "5".repeat(64),
            "6".repeat(64)
        );
        assert_eq!(bytes, expected.as_bytes());
        let decoded = decode_binding(&bytes, &context).unwrap();
        assert_eq!(decoded.source_sha, "9".repeat(40));
        assert!(matches!(
            decoded.preflight_environment,
            PreflightEnvironment::GithubHosted
        ));
    }
    #[test]
    fn budget_actor_duplicates_unknown_environment_and_types_fail_closed() {
        let context = context();
        let good: Value =
            serde_json::from_slice(&encode_binding(&binding(), &context).unwrap()).unwrap();
        for key in [
            "issued_utc_ms",
            "run_expires_utc_ms",
            "cleanup_expires_utc_ms",
            "parent_elapsed_ms_at_dispatch",
        ] {
            let mut bad = good.clone();
            bad["original_budget"][key] = json!(bad["original_budget"][key].as_u64().unwrap() + 1);
            assert!(decode_binding(&serde_json::to_vec(&bad).unwrap(), &context).is_err());
        }
        for (key, value) in [
            ("actor_sid", json!("S-1-5-21-1-2-3-1002")),
            ("preflight_environment", json!("local")),
            ("source_sha", json!("A".repeat(40))),
            ("context_sha256", json!(true)),
            ("run_nonce", json!("0".repeat(32))),
            ("authority_credential", json!("PRIVATE")),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(decode_binding(&serde_json::to_vec(&bad).unwrap(), &context).is_err());
        }
        assert!(decode_binding(b"{\"schema\":1,\"schema\":2}", &context).is_err());
        let oversized = vec![b' '; LIMIT + 1];
        assert!(decode_binding(&oversized, &context).is_err());
    }
    #[test]
    fn provenance_commitment_uses_typed_compact_order_without_newline() {
        let UnverifiedParentContext::Workload(context) = context() else {
            unreachable!()
        };
        let bytes = encode_provenance(&context.provenance).unwrap();
        let expected = format!(
            "{{\"managed\":{{\"source_sha\":\"{}\",\"required_source_sha\":\"{}\",\"receipt_sha256\":\"{}\"}},\"native\":{{\"source_sha\":\"{}\",\"cargo_lock_sha256\":\"{}\",\"receipt_sha256\":\"{}\"}},\"harness\":{{\"source_sha\":\"{}\",\"cargo_lock_sha256\":\"{}\",\"receipt_sha256\":\"{}\"}}}}",
            "a".repeat(40),
            "b".repeat(40),
            "c".repeat(64),
            "d".repeat(40),
            "e".repeat(64),
            "f".repeat(64),
            "1".repeat(40),
            "2".repeat(64),
            "3".repeat(64)
        );
        assert_eq!(bytes, expected.as_bytes());
        assert_ne!(bytes.last(), Some(&b'\n'));
    }
}
