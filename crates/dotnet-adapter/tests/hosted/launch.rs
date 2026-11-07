//! Pure consumer of the shared UNVERIFIED parent wire. No Command or OS IO.
use humanymous_platform::testing::parent_context::{
    self, UnverifiedLaunch, UnverifiedMappedBudget, UnverifiedParentContext, WireError,
};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

/// Syntax and one conserved clock mapping only. This cannot authorize a launch,
/// authenticate a parent or establish file/token/Job/cleanup proof.
struct UnverifiedHostLaunch {
    context: UnverifiedParentContext,
    budget: UnverifiedMappedBudget,
}
impl UnverifiedHostLaunch {
    fn decode(bytes: &[u8], wall_ms: u64, received: Instant) -> Result<Self, WireError> {
        let context = parent_context::decode(bytes)?;
        Self::from_context(context, wall_ms, received)
    }

    /// Compare byte commitments and supplied identity facts only. Successful
    /// decoding remains UNVERIFIED and grants no Native/HostIO launch authority.
    fn decode_bound_claims(
        context_bytes: &[u8],
        binding_bytes: &[u8],
        locator_digest: &str,
        actor_sid: &str,
        run_nonce: &str,
        wall_ms: u64,
        received: Instant,
    ) -> Result<Self, WireError> {
        // The sole shared decoder enforces bounds before hashing untrusted data.
        let context = parent_context::decode(context_bytes)?;
        let binding = parent_context::decode_binding(binding_bytes, &context)?;
        let UnverifiedParentContext::Workload(launch) = &context else {
            return Err(WireError);
        };
        let context_digest = format!("{:x}", Sha256::digest(context_bytes));
        let provenance_digest = format!(
            "{:x}",
            Sha256::digest(parent_context::encode_provenance(&launch.provenance)?)
        );
        if locator_digest != context_digest
            || binding.context_sha256 != context_digest
            || binding.provenance_sha256 != provenance_digest
            || binding.actor_sid != actor_sid
            || binding.run_nonce != run_nonce
        {
            return Err(WireError);
        }
        Self::from_context(context, wall_ms, received)
    }

    fn from_context(
        context: UnverifiedParentContext,
        wall_ms: u64,
        received: Instant,
    ) -> Result<Self, WireError> {
        if !matches!(context, UnverifiedParentContext::Workload(_)) {
            return Err(WireError);
        }
        let budget = context.map_budget(wall_ms, received)?;
        Ok(Self { context, budget })
    }

    fn claimed_paths(&self) -> &UnverifiedLaunch {
        match &self.context {
            UnverifiedParentContext::Workload(launch) => launch,
            UnverifiedParentContext::CleanupOnly(_) => unreachable!("constructor rejects cleanup"),
        }
    }

    fn claimed_environment(&self) -> Vec<(&'static str, String)> {
        self.context.fixed_environment()
    }

    fn stage_budget(&self, now: Instant, ceiling: Duration) -> Result<Duration, WireError> {
        self.budget
            .run_deadline()
            .and_then(|until| until.checked_duration_since(now))
            .map(|remaining| remaining.min(ceiling))
            .filter(|remaining| !remaining.is_zero())
            .ok_or(WireError)
    }

    fn cleanup_budget(&self, now: Instant) -> Result<Duration, WireError> {
        self.budget
            .cleanup_deadline()
            .checked_duration_since(now)
            .filter(|remaining| !remaining.is_zero())
            .ok_or(WireError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parent_context::{
        ManagedLineage, NativeLineage, PreflightEnvironment, UnverifiedBinding, UnverifiedBudget,
        UnverifiedCleanup, UnverifiedProvenance,
    };

    fn provenance() -> UnverifiedProvenance {
        let native = || NativeLineage {
            source_sha: "a".repeat(40),
            cargo_lock_sha256: "b".repeat(64),
            receipt_sha256: "c".repeat(64),
        };
        UnverifiedProvenance {
            managed: ManagedLineage {
                source_sha: "d".repeat(40),
                required_source_sha: "e".repeat(40),
                receipt_sha256: "f".repeat(64),
            },
            native: native(),
            harness: native(),
        }
    }
    fn budget(elapsed: u64) -> UnverifiedBudget {
        UnverifiedBudget {
            issued_utc_ms: 1000,
            run_expires_utc_ms: 181000,
            cleanup_expires_utc_ms: 241000,
            parent_elapsed_ms_at_dispatch: elapsed,
        }
    }
    fn workload() -> Vec<u8> {
        parent_context::encode(&UnverifiedParentContext::Workload(UnverifiedLaunch {
            apphost: "C:/app/Humanymous.HostPilot.exe".into(),
            app_root: "C:/app".into(),
            system_root: "C:/Windows".into(),
            runtime_root: "C:/runtime".into(),
            private_work: "C:/actor/work".into(),
            audit_root: "C:/actor/work/audit".into(),
            handoff_root: "C:/actor/handoff".into(),
            actor_sid: "S-1-5-21-1-2-3-1001".into(),
            provenance: provenance(),
            budget: budget(100),
        }))
        .unwrap()
    }

    fn binding(context_bytes: &[u8]) -> (UnverifiedParentContext, UnverifiedBinding) {
        let context = parent_context::decode(context_bytes).unwrap();
        let UnverifiedParentContext::Workload(launch) = &context else {
            unreachable!()
        };
        let binding = UnverifiedBinding {
            schema: "humanymous.parent-context-binding/1".into(),
            run_nonce: "1".repeat(32),
            actor_sid: launch.actor_sid.clone(),
            source_sha: "2".repeat(40),
            context_sha256: format!("{:x}", Sha256::digest(context_bytes)),
            verified_manifest_sha256: "3".repeat(64),
            provenance_sha256: format!(
                "{:x}",
                Sha256::digest(parent_context::encode_provenance(&launch.provenance).unwrap())
            ),
            original_budget: budget(100),
            preflight_environment: PreflightEnvironment::GithubHosted,
        };
        (context, binding)
    }

    #[test]
    fn consistent_shared_binding_stays_unverified_and_conserves_budget() {
        let bytes = workload();
        let (context, binding) = binding(&bytes);
        let encoded = parent_context::encode_binding(&binding, &context).unwrap();
        let received = Instant::now();
        let consumer = UnverifiedHostLaunch::decode_bound_claims(
            &bytes,
            &encoded,
            &binding.context_sha256,
            &binding.actor_sid,
            &binding.run_nonce,
            1100,
            received,
        )
        .unwrap();
        assert_eq!(consumer.claimed_paths().actor_sid, binding.actor_sid);
        assert!(
            consumer
                .stage_budget(
                    received + Duration::from_millis(179_900),
                    Duration::from_secs(15)
                )
                .is_err()
        );
    }

    #[test]
    fn locator_or_binding_context_digest_mismatch_fails_closed() {
        let bytes = workload();
        let (context, mut binding) = binding(&bytes);
        let encoded = parent_context::encode_binding(&binding, &context).unwrap();
        assert!(
            UnverifiedHostLaunch::decode_bound_claims(
                &bytes,
                &encoded,
                &"0".repeat(64),
                &binding.actor_sid,
                &binding.run_nonce,
                1100,
                Instant::now()
            )
            .is_err()
        );
        let actual_digest = binding.context_sha256.clone();
        binding.context_sha256 = "0".repeat(64);
        let encoded = parent_context::encode_binding(&binding, &context).unwrap();
        assert!(
            UnverifiedHostLaunch::decode_bound_claims(
                &bytes,
                &encoded,
                &actual_digest,
                &binding.actor_sid,
                &binding.run_nonce,
                1100,
                Instant::now()
            )
            .is_err()
        );
    }

    #[test]
    fn provenance_commitment_uses_shared_typed_bytes_without_newline() {
        let bytes = workload();
        let (context, mut binding) = binding(&bytes);
        let UnverifiedParentContext::Workload(launch) = &context else {
            unreachable!()
        };
        let mut wrong_bytes = parent_context::encode_provenance(&launch.provenance).unwrap();
        wrong_bytes.push(b'\n');
        binding.provenance_sha256 = format!("{:x}", Sha256::digest(wrong_bytes));
        let encoded = parent_context::encode_binding(&binding, &context).unwrap();
        assert!(
            UnverifiedHostLaunch::decode_bound_claims(
                &bytes,
                &encoded,
                &binding.context_sha256,
                &binding.actor_sid,
                &binding.run_nonce,
                1100,
                Instant::now()
            )
            .is_err()
        );
    }

    #[test]
    fn supplied_actor_and_namespace_facts_must_match_binding() {
        let bytes = workload();
        let (context, binding) = binding(&bytes);
        let encoded = parent_context::encode_binding(&binding, &context).unwrap();
        for (actor, nonce) in [
            ("S-1-5-21-1-2-3-1002".to_owned(), binding.run_nonce.clone()),
            (binding.actor_sid.clone(), "2".repeat(32)),
        ] {
            assert!(
                UnverifiedHostLaunch::decode_bound_claims(
                    &bytes,
                    &encoded,
                    &binding.context_sha256,
                    &actor,
                    &nonce,
                    1100,
                    Instant::now()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn shared_paths_and_environment_are_consumed_without_a_second_codec() {
        let bytes = workload();
        let source = parent_context::decode(&bytes).unwrap();
        let consumer = UnverifiedHostLaunch::decode(&bytes, 1100, Instant::now()).unwrap();
        assert_eq!(
            consumer.claimed_paths().apphost,
            "C:/app/Humanymous.HostPilot.exe"
        );
        assert_eq!(consumer.claimed_paths().audit_root, "C:/actor/work/audit");
        assert_eq!(consumer.claimed_environment(), source.fixed_environment());
        assert!(
            !consumer
                .claimed_environment()
                .iter()
                .any(|(key, _)| *key == "DOTNET_STARTUP_HOOKS")
        );
    }

    #[test]
    fn sequential_profiles_borrow_the_original_mapping_without_renewal() {
        let received = Instant::now();
        let consumer = UnverifiedHostLaunch::decode(&workload(), 1100, received).unwrap();
        assert_eq!(
            consumer
                .stage_budget(received, Duration::from_secs(15))
                .unwrap(),
            Duration::from_secs(15)
        );
        let later = received + Duration::from_millis(179_899);
        assert_eq!(
            consumer
                .stage_budget(later, Duration::from_secs(15))
                .unwrap(),
            Duration::from_millis(1)
        );
        assert!(
            consumer
                .stage_budget(later + Duration::from_millis(1), Duration::from_secs(15))
                .is_err()
        );
        assert_eq!(
            consumer
                .cleanup_budget(received + Duration::from_millis(179_900))
                .unwrap(),
            Duration::from_secs(60)
        );
        assert!(
            consumer
                .cleanup_budget(received + Duration::from_millis(239_900))
                .is_err()
        );
    }

    #[test]
    fn cleanup_only_context_never_becomes_a_workload_consumer() {
        let bytes =
            parent_context::encode(&UnverifiedParentContext::CleanupOnly(UnverifiedCleanup {
                system_root: "C:/Windows".into(),
                private_work: "C:/actor/work".into(),
                handoff_root: "C:/actor/handoff".into(),
                actor_sid: "S-1-5-21-1-2-3-1001".into(),
                provenance: provenance(),
                budget: budget(180000),
            }))
            .unwrap();
        assert!(UnverifiedHostLaunch::decode(&bytes, 181000, Instant::now()).is_err());
    }

    #[test]
    fn exhausted_or_zero_stage_budget_cannot_mint_more_time() {
        let received = Instant::now();
        assert!(UnverifiedHostLaunch::decode(&workload(), 181000, received).is_err());
        let consumer = UnverifiedHostLaunch::decode(&workload(), 1100, received).unwrap();
        assert!(consumer.stage_budget(received, Duration::ZERO).is_err());
        assert!(UnverifiedHostLaunch::decode(b"{}", 1100, received).is_err());
    }
}
