//! `/2` host launch context with an OS-tagged actor (SoT dotnet-protected-host
//! launch, L3). `/1` bytes stay frozen in the parent module. Decoding NEVER
//! authenticates a parent, actor, path, scope, Job or deadline.
use super::{
    PreflightEnvironment, UnverifiedBudget, UnverifiedMappedBudget, UnverifiedProvenance,
    WireError, bounded, hex, map_original, provenance, valid_budget,
};
use crate::testing::json::decode_strict_json;
use serde::{Deserialize, Serialize};
use std::time::Instant;

const CONTEXT_SCHEMA: &str = "humanymous.dotnet-host-launch/2";
const BINDING_SCHEMA: &str = "humanymous.parent-context-binding/2";
const BINDING_LIMIT: usize = 16 * 1024;

/// Claimed actor identity, tagged by OS. Never a substitute for the actual
/// TokenUser SID (Windows) or the actual real/effective uid (Linux).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "os", deny_unknown_fields)]
pub enum ActorIdentity {
    /// Linux hosted runner user; uid 0 is never valid.
    #[serde(rename = "linux")]
    Linux {
        /// Claimed real uid.
        uid: u32,
    },
    /// Windows per-run reserved account.
    #[serde(rename = "windows")]
    Windows {
        /// Claimed account SID.
        sid: String,
    },
}
impl ActorIdentity {
    fn valid(&self) -> bool {
        match self {
            Self::Linux { uid } => *uid != 0,
            Self::Windows { sid } => bounded(sid) && sid.starts_with("S-1-"),
        }
    }
}

/// Unauthenticated `/2` workload context. Paths are bounded text only.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedHostContext {
    /// Must equal `humanymous.dotnet-host-launch/2`.
    pub schema: String,
    /// Claimed fixed apphost path.
    pub apphost: String,
    /// Claimed protected app root.
    pub app_root: String,
    /// Claimed runtime root (DOTNET_ROOT).
    pub runtime_root: String,
    /// Claimed Windows system root; present exactly for a Windows actor.
    pub system_root: Option<String>,
    /// Claimed actor-private working directory.
    pub private_work: String,
    /// Claimed private audit directory.
    pub audit_root: String,
    /// Claimed restricted outcome handoff directory.
    pub handoff_root: String,
    /// Claimed OS-tagged actor.
    pub actor: ActorIdentity,
    /// Unverified named build metadata.
    pub provenance: UnverifiedProvenance,
    /// Original parent clock claims.
    pub budget: UnverifiedBudget,
}
impl UnverifiedHostContext {
    fn validate(&self) -> Result<(), WireError> {
        let windows = matches!(self.actor, ActorIdentity::Windows { .. });
        if self.schema != CONTEXT_SCHEMA
            || ![
                &self.apphost,
                &self.app_root,
                &self.runtime_root,
                &self.private_work,
                &self.audit_root,
                &self.handoff_root,
            ]
            .into_iter()
            .all(|value| bounded(value))
            || self
                .system_root
                .as_deref()
                .is_some_and(|root| !bounded(root))
            || self.system_root.is_some() != windows
            || !self.actor.valid()
            || !provenance(&self.provenance)
            || !valid_budget(&self.budget, false)
        {
            return Err(WireError);
        }
        Ok(())
    }
    /// Fixed keys/values from UNVERIFIED text, applied only after env_clear and
    /// native admission. No PATH on Linux, no startup hook, no probing path.
    pub fn fixed_environment(&self) -> Vec<(&'static str, String)> {
        let work = &self.private_work;
        let mut env = match &self.system_root {
            Some(system) => vec![
                ("SystemRoot", system.clone()),
                ("WINDIR", system.clone()),
                ("PATH", format!("{system}/System32")),
                ("TEMP", format!("{work}/temp")),
                ("TMP", format!("{work}/temp")),
                ("USERPROFILE", format!("{work}/profile")),
            ],
            None => vec![
                ("HOME", format!("{work}/profile")),
                ("TMPDIR", format!("{work}/temp")),
            ],
        };
        env.extend([
            ("DOTNET_ROOT", self.runtime_root.clone()),
            ("DOTNET_ROOT_X64", self.runtime_root.clone()),
            ("DOTNET_MULTILEVEL_LOOKUP", "0".into()),
            ("DOTNET_EnableDiagnostics", "0".into()),
            ("DOTNET_ROLL_FORWARD", "LatestPatch".into()),
        ]);
        env
    }
    /// Map once at receipt; every profile and derived mode borrows this mapping.
    pub fn map_budget(
        &self,
        wall_ms: u64,
        received: Instant,
    ) -> Result<UnverifiedMappedBudget, WireError> {
        self.validate()?;
        map_original(&self.budget, false, wall_ms, received)
    }
}

/// Unauthenticated `/2` binding. Equality and hashes never authenticate a parent.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedHostBinding {
    /// Must equal `humanymous.parent-context-binding/2`.
    pub schema: String,
    /// Claimed original run nonce, canonical nonzero lowercase hex32.
    pub run_nonce: String,
    /// Claimed OS-tagged actor; must equal the context actor.
    pub actor: ActorIdentity,
    /// Producer/invocation commit only.
    pub source_sha: String,
    /// Digest of the exact context wire bytes, checked by the consumer.
    pub context_sha256: String,
    /// Independent verified-manifest commitment.
    pub verified_manifest_sha256: String,
    /// Digest of `encode_provenance` bytes.
    pub provenance_sha256: String,
    /// Exact original context budget.
    pub original_budget: UnverifiedBudget,
    /// Venue metadata only.
    pub preflight_environment: PreflightEnvironment,
}
impl UnverifiedHostBinding {
    fn validate(&self, context: &UnverifiedHostContext) -> Result<(), WireError> {
        context.validate()?;
        if self.schema != BINDING_SCHEMA
            || !nonce(&self.run_nonce)
            || self.actor != context.actor
            || !hex(&self.source_sha, 40)
            || !hex(&self.context_sha256, 64)
            || !hex(&self.verified_manifest_sha256, 64)
            || !hex(&self.provenance_sha256, 64)
            || self.original_budget != context.budget
        {
            return Err(WireError);
        }
        Ok(())
    }
}

/// Canonical nonzero lowercase hex32 run nonce.
pub fn nonce(value: &str) -> bool {
    hex(value, 32) && value.bytes().any(|byte| byte != b'0')
}

/// Strict bounded `/2` decode. Rejects `/1` and cleanup-only schemas.
pub fn decode_host(bytes: &[u8]) -> Result<UnverifiedHostContext, WireError> {
    let value = decode_strict_json(bytes, super::MAX_BYTES).map_err(|_| WireError)?;
    let context: UnverifiedHostContext = serde_json::from_value(value).map_err(|_| WireError)?;
    context.validate()?;
    Ok(context)
}
/// Deterministic compact JSON plus LF.
pub fn encode_host(context: &UnverifiedHostContext) -> Result<Vec<u8>, WireError> {
    context.validate()?;
    let mut bytes = serde_json::to_vec(context).map_err(|_| WireError)?;
    bytes.push(b'\n');
    if bytes.len() > super::MAX_BYTES {
        return Err(WireError);
    }
    Ok(bytes)
}
/// Strict bounded binding decode with exact actor/budget comparison only.
pub fn decode_host_binding(
    bytes: &[u8],
    context: &UnverifiedHostContext,
) -> Result<UnverifiedHostBinding, WireError> {
    let value = decode_strict_json(bytes, BINDING_LIMIT).map_err(|_| WireError)?;
    let binding: UnverifiedHostBinding = serde_json::from_value(value).map_err(|_| WireError)?;
    binding.validate(context)?;
    Ok(binding)
}
/// Encode fixed typed fields in declaration order, compact JSON plus LF.
pub fn encode_host_binding(
    binding: &UnverifiedHostBinding,
    context: &UnverifiedHostContext,
) -> Result<Vec<u8>, WireError> {
    binding.validate(context)?;
    let mut bytes = serde_json::to_vec(binding).map_err(|_| WireError)?;
    bytes.push(b'\n');
    if bytes.len() > BINDING_LIMIT {
        return Err(WireError);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::time::Duration;

    fn wire(actor: Value, system_root: Value) -> Value {
        json!({"schema":"humanymous.dotnet-host-launch/2","apphost":"/srv/app/Humanymous.HostPilot","app_root":"/srv/app","runtime_root":"/srv/runtime","system_root":system_root,"private_work":"/srv/work","audit_root":"/srv/work/audit","handoff_root":"/srv/handoff","actor":actor,
        "provenance":{"managed":{"source_sha":"a".repeat(40),"required_source_sha":"b".repeat(40),"receipt_sha256":"c".repeat(64)},"native":{"source_sha":"d".repeat(40),"cargo_lock_sha256":"e".repeat(64),"receipt_sha256":"f".repeat(64)},"harness":{"source_sha":"1".repeat(40),"cargo_lock_sha256":"2".repeat(64),"receipt_sha256":"3".repeat(64)}},
        "budget":{"issued_utc_ms":1000,"run_expires_utc_ms":181000,"cleanup_expires_utc_ms":241000,"parent_elapsed_ms_at_dispatch":100}})
    }
    fn linux() -> Value {
        wire(json!({"os":"linux","uid":1000}), Value::Null)
    }
    fn parsed(value: &Value) -> Result<UnverifiedHostContext, WireError> {
        decode_host(&serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn os_tagged_actor_round_trips_and_v1_bytes_are_refused() {
        let context = parsed(&linux()).unwrap();
        let bytes = encode_host(&context).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), linux());
        assert!(bytes.ends_with(b"}\n"));
        let windows = wire(
            json!({"os":"windows","sid":"S-1-5-21-1-2-3-1001"}),
            json!("C:/Windows"),
        );
        assert!(parsed(&windows).is_ok());
        let mut v1 = linux();
        v1["schema"] = json!("humanymous.dotnet-prebuilt-launch/1");
        assert!(parsed(&v1).is_err());
        assert!(super::super::decode(&serde_json::to_vec(&linux()).unwrap()).is_err());
    }

    #[test]
    fn root_unknown_os_and_mismatched_system_root_fail_closed() {
        for bad in [
            wire(json!({"os":"linux","uid":0}), Value::Null),
            wire(json!({"os":"macos","uid":501}), Value::Null),
            wire(json!({"os":"linux","uid":1000}), json!("C:/Windows")),
            wire(json!({"os":"windows","sid":"S-1-5-18"}), Value::Null),
            wire(
                json!({"os":"windows","sid":"not-a-sid"}),
                json!("C:/Windows"),
            ),
            wire(json!({"os":"linux","uid":1000,"sid":"S-1-1"}), Value::Null),
        ] {
            assert!(parsed(&bad).is_err());
        }
        let mut bad = linux();
        bad["apphost"] = json!("/srv/app/\nhost");
        assert!(parsed(&bad).is_err());
        let mut bad = linux();
        bad["budget"]["run_expires_utc_ms"] = json!(181001);
        assert!(parsed(&bad).is_err());
        let mut bad = linux();
        bad["extra"] = json!(1);
        assert!(parsed(&bad).is_err());
    }

    #[test]
    fn linux_environment_has_no_path_and_windows_keeps_system_keys() {
        let env = parsed(&linux()).unwrap().fixed_environment();
        assert_eq!(env.len(), 7);
        assert!(env.iter().all(|(key, _)| !matches!(
            *key,
            "PATH" | "DOTNET_STARTUP_HOOKS" | "LD_PRELOAD" | "SystemRoot"
        )));
        assert!(env.contains(&("HOME", "/srv/work/profile".into())));
        let windows = parsed(&wire(
            json!({"os":"windows","sid":"S-1-5-21-1-2-3-1001"}),
            json!("C:/Windows"),
        ))
        .unwrap()
        .fixed_environment();
        assert_eq!(windows.len(), 11);
        assert!(windows.contains(&("PATH", "C:/Windows/System32".into())));
    }

    #[test]
    fn binding_actor_budget_and_nonce_are_exact() {
        let context = parsed(&linux()).unwrap();
        let binding = || UnverifiedHostBinding {
            schema: BINDING_SCHEMA.into(),
            run_nonce: "a".repeat(32),
            actor: ActorIdentity::Linux { uid: 1000 },
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
        };
        let bytes = encode_host_binding(&binding(), &context).unwrap();
        assert!(decode_host_binding(&bytes, &context).is_ok());
        let mut wrong = binding();
        wrong.actor = ActorIdentity::Linux { uid: 1001 };
        assert!(encode_host_binding(&wrong, &context).is_err());
        let mut wrong = binding();
        wrong.run_nonce = "0".repeat(32);
        assert!(encode_host_binding(&wrong, &context).is_err());
        let mut wrong = binding();
        wrong.original_budget.parent_elapsed_ms_at_dispatch = 101;
        assert!(encode_host_binding(&wrong, &context).is_err());
        let text = String::from_utf8(bytes).unwrap();
        let duplicate = text.replacen("{\"schema\"", "{\"schema\":\"x\",\"schema\"", 1);
        assert!(decode_host_binding(duplicate.as_bytes(), &context).is_err());
    }

    #[test]
    fn budget_maps_once_and_never_renews() {
        let now = Instant::now();
        let context = parsed(&linux()).unwrap();
        let budget = context.map_budget(1200, now).unwrap();
        assert_eq!(
            budget.run_deadline().unwrap().duration_since(now),
            Duration::from_millis(179800)
        );
        assert!(context.map_budget(181000, now).is_err());
    }
}
