use crate::Host;
use serde::{Deserialize, Serialize};

/// Limited initial corpus identity, never native SDK or all-framework support.
pub const QUALIFICATION_PROFILE: &str = "humanymous.go-gate/bounded-http-8-case/1";

/// Required actual host and isolated Gate observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Case {
    /// Allowed bounded binary request arrives byte-identical exactly once.
    BinaryAllow,
    /// Authority or required evidence is unavailable; origin stays untouched.
    DependencyDown,
    /// Request cancellation prevents a later protected origin effect.
    Cancellation,
    /// Oversized/incomplete input cannot reach the application parser.
    IncompleteBody,
    /// Forged forwarding/capture headers do not gain trust.
    ForgedProvenance,
    /// Direct origin access from attacker namespace is denied.
    OriginBypass,
    /// Upgrade and CONNECT cannot escape bounded HTTP authorization.
    Upgrade,
    /// Response duplicate cookies and HEAD/bodyless semantics survive.
    ResponseSemantics,
}
const REQUIRED: [Case; 8] = [
    Case::BinaryAllow,
    Case::DependencyDown,
    Case::Cancellation,
    Case::IncompleteBody,
    Case::ForgedProvenance,
    Case::OriginBypass,
    Case::Upgrade,
    Case::ResponseSemantics,
];

/// Independently selected immutable execution identity; never derived from receipt.
pub struct ExpectedTuple<'a> {
    /// Exact framework family.
    pub host: Host,
    /// Runtime/application source commit.
    pub source: &'a str,
    /// Runtime container digest, without sha256 prefix.
    pub image: &'a str,
    /// Complete external fixture artifact digest, not merely entrypoint source.
    pub fixture: &'a str,
    /// Sanitized configuration identity, including route requirements.
    pub configuration: &'a str,
    /// Frozen engine configuration/source identity.
    pub engine: &'a str,
    /// Exact measured upstream framework version.
    pub host_version: &'a str,
    /// Exact measured Go compiler/runtime version.
    pub go_version: &'a str,
    /// Actual runner operating system.
    pub os: &'a str,
    /// Actual runner architecture.
    pub arch: &'a str,
}

/// Exact case accounting; absent/ignored/mock results cannot qualify a tuple.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    /// Case identity, unique in the receipt.
    pub case: Case,
    /// Actual executed assertions, excluding ignored cases.
    pub assertions: u32,
    /// Failed assertions are retained, never removed from the denominator.
    pub failures: u32,
    /// Independent protected application effect counter.
    pub origin_effects: u32,
    /// True only for real host execution; false for synthetic/component fixtures.
    pub actual_host: bool,
}
/// Source-bound receipt checker, not an execution or attestation service.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Exact bounded corpus version; unknown/broader claims reject.
    pub profile: String,
    /// Exact host family; sibling frameworks do not inherit qualification.
    pub host: Host,
    /// Immutable full Git source SHA.
    pub source_sha: String,
    /// Exact pinned container image digest.
    pub image_sha256: String,
    /// Pinned external Go fixture source digest.
    pub fixture_sha256: String,
    /// Complete sanitized route/configuration digest.
    pub configuration_sha256: String,
    /// Frozen engine identity digest.
    pub engine_sha256: String,
    /// Actual upstream host version.
    pub host_version: String,
    /// Actual Go compiler/runtime version.
    pub go_version: String,
    /// Actual runner operating system.
    pub os: String,
    /// Actual runner architecture.
    pub arch: String,
    /// Isolated Docker product topology, measured by independent lab witness.
    pub isolated_docker: bool,
    /// Standard free hosted runner execution.
    pub hosted: bool,
    /// All required case results, including failures.
    pub outcomes: Vec<Outcome>,
}
impl Receipt {
    /// Require the expected immutable tuple and complete independent accounting.
    /// A caller-provided receipt alone does not establish authentic execution.
    pub fn qualifies(&self, expected: &ExpectedTuple<'_>) -> bool {
        let hex = |s: &str, n| {
            s.len() == n
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        let label = |s: &str| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        };
        self.profile == QUALIFICATION_PROFILE
            && self.host == expected.host
            && self.source_sha == expected.source
            && hex(expected.source, 40)
            && [
                &self.image_sha256,
                &self.fixture_sha256,
                &self.configuration_sha256,
                &self.engine_sha256,
            ]
            .into_iter()
            .zip([
                expected.image,
                expected.fixture,
                expected.configuration,
                expected.engine,
            ])
            .all(|(actual, wanted)| actual == wanted && hex(wanted, 64))
            && [&self.host_version, &self.go_version, &self.os, &self.arch]
                .into_iter()
                .zip([
                    expected.host_version,
                    expected.go_version,
                    expected.os,
                    expected.arch,
                ])
                .all(|(actual, wanted)| actual == wanted && label(wanted))
            && self.isolated_docker
            && self.hosted
            && self.outcomes.len() == REQUIRED.len()
            && REQUIRED.into_iter().all(|case| {
                let mut found = self.outcomes.iter().filter(|r| r.case == case);
                let Some(r) = found.next() else {
                    return false;
                };
                found.next().is_none()
                    && r.actual_host
                    && r.assertions > 0
                    && r.failures == 0
                    && r.origin_effects
                        == u32::from(matches!(case, Case::BinaryAllow | Case::ResponseSemantics))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn receipt() -> Receipt {
        Receipt {
            profile: QUALIFICATION_PROFILE.into(),
            host: Host::NetHttp,
            source_sha: "a".repeat(40),
            image_sha256: "b".repeat(64),
            fixture_sha256: "c".repeat(64),
            configuration_sha256: "d".repeat(64),
            engine_sha256: "e".repeat(64),
            host_version: "stdlib".into(),
            go_version: "go-fixture-unit".into(),
            os: "linux".into(),
            arch: "amd64".into(),
            isolated_docker: true,
            hosted: true,
            outcomes: REQUIRED
                .into_iter()
                .map(|case| Outcome {
                    case,
                    assertions: 1,
                    failures: 0,
                    origin_effects: u32::from(matches!(
                        case,
                        Case::BinaryAllow | Case::ResponseSemantics
                    )),
                    actual_host: true,
                })
                .collect(),
        }
    }
    fn qualifies(r: &Receipt) -> bool {
        r.qualifies(&ExpectedTuple {
            host: Host::NetHttp,
            source: &"a".repeat(40),
            image: &"b".repeat(64),
            fixture: &"c".repeat(64),
            configuration: &"d".repeat(64),
            engine: &"e".repeat(64),
            host_version: "stdlib",
            go_version: "go-fixture-unit",
            os: "linux",
            arch: "amd64",
        })
    }
    #[test]
    fn requires_exact_source_host_topology_and_real_execution() {
        let mut r = receipt();
        assert!(qualifies(&r));
        r.host = Host::Fiber;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.hosted = false;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.isolated_docker = false;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.outcomes[0].actual_host = false;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.fixture_sha256 = "d".repeat(64);
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.configuration_sha256 = "f".repeat(64);
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.engine_sha256 = "f".repeat(64);
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.go_version = "other".into();
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.os = "windows".into();
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.profile = "native-go-sdk".into();
        assert!(!qualifies(&r));
    }
    #[test]
    fn missing_duplicate_failed_and_unexpected_effects_reject() {
        let mut r = receipt();
        r.outcomes.pop();
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.outcomes[1].case = Case::BinaryAllow;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.outcomes[1].origin_effects = 1;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.outcomes[1].failures = 1;
        assert!(!qualifies(&r));
        let mut r = receipt();
        r.outcomes[1].assertions = 0;
        assert!(!qualifies(&r));
    }
}
