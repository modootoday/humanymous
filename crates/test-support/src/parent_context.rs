//! Shared parent-context syntax, fixed environment and conserved budget mapping.
//! Decoding NEVER authenticates a parent, artifact, token, path, Job, Ready state
//! or deadline. Native consumers must retain the original parent watchdog/owner.
use crate::json::decode_strict_json;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{Duration, Instant},
};

/// Maximum wire envelope bytes, before JSON allocation.
pub const MAX_BYTES: usize = 64 * 1024;
/// Closed redacted codec error; raw input/remote errors never enter diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireError;
impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid unverified parent context")
    }
}
impl std::error::Error for WireError {}

/// Unauthenticated source metadata, not original build provenance.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedLineage {
    /// Claimed compiled source SHA, syntax checked only.
    pub source_sha: String,
    /// Claimed required managed-source SHA.
    pub required_source_sha: String,
    /// Claimed independently retained receipt digest.
    pub receipt_sha256: String,
}
/// Unauthenticated native or harness build metadata.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeLineage {
    /// Claimed compiled source SHA.
    pub source_sha: String,
    /// Claimed Cargo lock digest.
    pub cargo_lock_sha256: String,
    /// Claimed build receipt digest.
    pub receipt_sha256: String,
}
/// Distinct named lineages; no invented common atomic source.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedProvenance {
    /// Managed build metadata.
    pub managed: ManagedLineage,
    /// Native facade metadata.
    pub native: NativeLineage,
    /// Rust harness metadata.
    pub harness: NativeLineage,
}
/// Parent-claimed original clock envelope, never authoritative native evidence.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedBudget {
    /// Original UTC milliseconds at parent budget creation.
    pub issued_utc_ms: u64,
    /// Original run expiry, exactly issued+180000.
    pub run_expires_utc_ms: u64,
    /// Original cleanup expiry, exactly run+60000.
    pub cleanup_expires_utc_ms: u64,
    /// Claimed parent monotonic elapsed at dispatch; cannot prove transit time.
    pub parent_elapsed_ms_at_dispatch: u64,
}
/// Unauthenticated workload context. Paths are bounded text, not native proof.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedLaunch {
    /// Claimed fixed apphost path; the runner validates actual artifacts.
    pub apphost: String,
    /// Claimed protected app root.
    pub app_root: String,
    /// Claimed Windows system root.
    pub system_root: String,
    /// Claimed protected runtime root.
    pub runtime_root: String,
    /// Claimed actor-private working directory.
    pub private_work: String,
    /// Claimed private audit directory.
    pub audit_root: String,
    /// Claimed restricted outcome handoff directory.
    pub handoff_root: String,
    /// Claimed actor SID; never a substitute for actual TokenUser.
    pub actor_sid: String,
    /// Unverified named build metadata.
    pub provenance: UnverifiedProvenance,
    /// Original parent clock claims.
    pub budget: UnverifiedBudget,
}
/// Cleanup context intentionally has no apphost, harness or workload launch data.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedCleanup {
    /// Claimed Windows system root.
    pub system_root: String,
    /// Claimed exact actor-private cleanup root.
    pub private_work: String,
    /// Claimed restricted cleanup handoff directory.
    pub handoff_root: String,
    /// Claimed original generated actor SID.
    pub actor_sid: String,
    /// Unverified original build metadata.
    pub provenance: UnverifiedProvenance,
    /// Original cleanup reserve, never renewed at cleanup-child startup.
    pub budget: UnverifiedBudget,
}
/// Strict tagged syntax only. Cleanup decoding does NOT authorize deletion.
#[derive(Serialize, Deserialize)]
#[serde(tag = "schema")]
pub enum UnverifiedParentContext {
    /// Claimed workload launch envelope.
    #[serde(rename = "humanymous.dotnet-prebuilt-launch/1")]
    Workload(UnverifiedLaunch),
    /// Claimed cleanup-only envelope.
    #[serde(rename = "humanymous.dotnet-prebuilt-cleanup/1")]
    CleanupOnly(UnverifiedCleanup),
}
fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn provenance(value: &UnverifiedProvenance) -> bool {
    hex(&value.managed.source_sha, 40)
        && hex(&value.managed.required_source_sha, 40)
        && hex(&value.managed.receipt_sha256, 64)
        && [&value.native, &value.harness].iter().all(|role| {
            hex(&role.source_sha, 40)
                && hex(&role.cargo_lock_sha256, 64)
                && hex(&role.receipt_sha256, 64)
        })
}
impl UnverifiedProvenance {
    /// Validate only named metadata syntax; never original build custody.
    pub fn validate_syntax(&self) -> Result<(), WireError> {
        if provenance(self) {
            Ok(())
        } else {
            Err(WireError)
        }
    }
}
impl UnverifiedLaunch {
    /// Fixed environment from UNVERIFIED text; native consumer admission required.
    pub fn fixed_environment(&self) -> Vec<(&'static str, String)> {
        fixed_environment(
            &self.system_root,
            &self.private_work,
            Some(&self.runtime_root),
        )
    }
}
fn fixed_environment(
    system: &str,
    work: &str,
    runtime: Option<&str>,
) -> Vec<(&'static str, String)> {
    let mut env = vec![
        ("SystemRoot", system.into()),
        ("WINDIR", system.into()),
        ("PATH", format!("{system}/System32")),
        ("TEMP", format!("{work}/temp")),
        ("TMP", format!("{work}/temp")),
        ("USERPROFILE", format!("{work}/profile")),
    ];
    if let Some(runtime) = runtime {
        env.extend([
            ("DOTNET_ROOT", runtime.into()),
            ("DOTNET_ROOT_X64", runtime.into()),
            ("DOTNET_MULTILEVEL_LOOKUP", "0".into()),
            ("DOTNET_EnableDiagnostics", "0".into()),
            ("DOTNET_ROLL_FORWARD", "LatestPatch".into()),
        ]);
    }
    env
}
impl UnverifiedParentContext {
    fn validate(&self) -> Result<(), WireError> {
        let (paths, sid, source, budget, cleanup) = match self {
            Self::Workload(v) => (
                vec![
                    &v.apphost,
                    &v.app_root,
                    &v.system_root,
                    &v.runtime_root,
                    &v.private_work,
                    &v.audit_root,
                    &v.handoff_root,
                ],
                &v.actor_sid,
                &v.provenance,
                &v.budget,
                false,
            ),
            Self::CleanupOnly(v) => (
                vec![&v.system_root, &v.private_work, &v.handoff_root],
                &v.actor_sid,
                &v.provenance,
                &v.budget,
                true,
            ),
        };
        if !paths.into_iter().all(|value| bounded(value))
            || !bounded(sid)
            || !provenance(source)
            || budget.run_expires_utc_ms.checked_sub(budget.issued_utc_ms) != Some(180000)
            || budget
                .cleanup_expires_utc_ms
                .checked_sub(budget.run_expires_utc_ms)
                != Some(60000)
            || budget.parent_elapsed_ms_at_dispatch >= if cleanup { 240000 } else { 180000 }
        {
            return Err(WireError);
        }
        Ok(())
    }
    /// Produce only fixed keys/values from UNVERIFIED text. Caller MUST establish
    /// actual trusted/native context before applying this after Command::env_clear.
    pub fn fixed_environment(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::Workload(value) => value.fixed_environment(),
            Self::CleanupOnly(value) => {
                fixed_environment(&value.system_root, &value.private_work, None)
            }
        }
    }
    /// Map once at receipt; consumers borrow this same unverified mapping. Actual
    /// original parent monotonic watchdog remains authoritative over transit/skew.
    pub fn map_budget(
        &self,
        wall_ms: u64,
        received: Instant,
    ) -> Result<UnverifiedMappedBudget, WireError> {
        self.validate()?;
        let (budget, cleanup_only) = match self {
            Self::Workload(v) => (&v.budget, false),
            Self::CleanupOnly(v) => (&v.budget, true),
        };
        let elapsed = budget.parent_elapsed_ms_at_dispatch;
        if wall_ms
            .checked_sub(budget.issued_utc_ms)
            .is_none_or(|wall| wall < elapsed)
        {
            return Err(WireError);
        }
        let cleanup = budget
            .cleanup_expires_utc_ms
            .checked_sub(wall_ms)
            .ok_or(WireError)?
            .min(240000 - elapsed);
        if cleanup == 0 {
            return Err(WireError);
        }
        let run_until = if cleanup_only {
            None
        } else {
            let run = budget
                .run_expires_utc_ms
                .checked_sub(wall_ms)
                .ok_or(WireError)?
                .min(180000 - elapsed);
            if run == 0 {
                return Err(WireError);
            }
            Some(
                received
                    .checked_add(Duration::from_millis(run))
                    .ok_or(WireError)?,
            )
        };
        Ok(UnverifiedMappedBudget {
            run_until,
            cleanup_until: received
                .checked_add(Duration::from_millis(cleanup))
                .ok_or(WireError)?,
        })
    }
}
/// Single local mapping of unverified clock claims, never native authorization.
/// No Clone/Deserialize; keep one object across sequential profiles.
pub struct UnverifiedMappedBudget {
    run_until: Option<Instant>,
    cleanup_until: Instant,
}
impl UnverifiedMappedBudget {
    /// Claimed local run deadline; absent for cleanup-only envelopes.
    pub fn run_deadline(&self) -> Option<Instant> {
        self.run_until
    }
    /// Claimed local cleanup deadline; actual original parent watchdog dominates.
    pub fn cleanup_deadline(&self) -> Instant {
        self.cleanup_until
    }
}
/// Reject duplicate/unknown/oversized/malformed syntax without echoing its bytes.
pub fn decode(bytes: &[u8]) -> Result<UnverifiedParentContext, WireError> {
    let value = decode_strict_json(bytes, MAX_BYTES).map_err(|_| WireError)?;
    let context: UnverifiedParentContext = serde_json::from_value(value).map_err(|_| WireError)?;
    context.validate()?;
    Ok(context)
}
/// Deterministic compact serde JSON plus LF; size/shape checked before return.
pub fn encode(context: &UnverifiedParentContext) -> Result<Vec<u8>, WireError> {
    context.validate()?;
    let mut bytes = serde_json::to_vec(context).map_err(|_| WireError)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_BYTES {
        return Err(WireError);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
