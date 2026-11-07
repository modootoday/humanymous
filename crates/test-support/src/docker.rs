//! Pure comparisons of caller-observed engines; this module never runs Docker.
//!
//! Obtain the baseline through the explicitly selected Windows Desktop client.
//! Query SDK identity through the actual SDK connection, not a separate CLI.
//! Equality binds observations; it does not authenticate their provenance.
use serde_json::{Value, json};
use std::fmt;

pub const DESKTOP_CONTEXT: &str = "desktop-linux";
pub const DESKTOP_ENDPOINT: &str = "npipe:////./pipe/dockerDesktopLinuxEngine";

/// Private execution metadata. Intentionally does not implement Debug/Serialize.
#[derive(Clone, Copy)]
pub struct ServerIdentity<'a> {
    pub server_id: &'a str,
    pub server_version: &'a str,
    pub os_type: &'a str,
}

#[derive(Clone, Copy)]
pub struct DesktopObservation<'a> {
    pub context: &'a str,
    pub endpoint: &'a str,
    pub server: ServerIdentity<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityError {
    MissingOrInvalidMetadata,
    WrongContext,
    WrongDesktopEndpoint,
    NonLinuxServer,
    UnexpectedEngine,
    UnexpectedVersion,
    MissingExplicitSdkHost,
    UnexpectedSdkHost,
}

impl fmt::Display for AffinityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingOrInvalidMetadata => "engine metadata missing or invalid",
            Self::WrongContext => "explicit Desktop context required",
            Self::WrongDesktopEndpoint => "expected Desktop endpoint required",
            Self::NonLinuxServer => "Linux container engine required",
            Self::UnexpectedEngine => "engine identity changed; resolve before mutation",
            Self::UnexpectedVersion => "engine version changed; resolve before mutation",
            Self::MissingExplicitSdkHost => "explicit allocated SDK host required",
            Self::UnexpectedSdkHost => "SDK host differs from explicit allocation",
        })
    }
}
impl std::error::Error for AffinityError {}

/// Constructed only after all comparisons pass; contains no private identifiers.
#[derive(Debug, Clone, Copy)]
pub struct AffinityProof {
    sdk: bool,
}
impl AffinityProof {
    pub fn summary(&self) -> Value {
        json!({"schema":"humanymous.docker-engine-affinity/1", "qualification":"observed-engine-binding", "engine_identity_matches":true, "server_version_matches":true, "linux_container_os":true, "explicit_endpoint_matches":true, "sdk_connection":self.sdk, "authenticated_provenance":false})
    }
}

fn metadata(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && value.bytes().all(|byte| byte.is_ascii_graphic())
}
fn server(value: &ServerIdentity<'_>) -> Result<(), AffinityError> {
    if !metadata(value.server_id, 256) || !metadata(value.server_version, 128) {
        return Err(AffinityError::MissingOrInvalidMetadata);
    }
    if value.os_type != "linux" {
        return Err(AffinityError::NonLinuxServer);
    }
    Ok(())
}
fn desktop(value: &DesktopObservation<'_>) -> Result<(), AffinityError> {
    if value.context != DESKTOP_CONTEXT {
        return Err(AffinityError::WrongContext);
    }
    if value.endpoint != DESKTOP_ENDPOINT {
        return Err(AffinityError::WrongDesktopEndpoint);
    }
    server(&value.server)
}
fn same_server(
    expected: &ServerIdentity<'_>,
    actual: &ServerIdentity<'_>,
) -> Result<(), AffinityError> {
    server(actual)?;
    if expected.server_id != actual.server_id {
        return Err(AffinityError::UnexpectedEngine);
    }
    if expected.server_version != actual.server_version {
        return Err(AffinityError::UnexpectedVersion);
    }
    Ok(())
}

pub fn verify_desktop(
    expected: &DesktopObservation<'_>,
    actual: &DesktopObservation<'_>,
) -> Result<AffinityProof, AffinityError> {
    desktop(expected)?;
    desktop(actual)?;
    same_server(&expected.server, &actual.server)?;
    Ok(AffinityProof { sdk: false })
}

/// The allocated host comes from explicit execution policy, never ambient fallback.
/// The caller supplies both the host actually configured on the SDK and the server
/// actually queried through that connection. No environment variables are read.
pub fn verify_sdk(
    expected: &DesktopObservation<'_>,
    allocated_host: &str,
    configured_host: Option<&str>,
    queried_server: &ServerIdentity<'_>,
) -> Result<AffinityProof, AffinityError> {
    allocated_connection(expected, allocated_host, configured_host, queried_server)?;
    Ok(AffinityProof { sdk: true })
}

/// Compare an explicitly allocated CLI proxy without claiming an SDK query.
/// The caller must query through the CLI connection it actually configured.
pub fn verify_allocated_cli(
    expected: &DesktopObservation<'_>,
    allocated_host: &str,
    configured_host: Option<&str>,
    queried_server: &ServerIdentity<'_>,
) -> Result<AffinityProof, AffinityError> {
    allocated_connection(expected, allocated_host, configured_host, queried_server)?;
    Ok(AffinityProof { sdk: false })
}

fn allocated_connection(
    expected: &DesktopObservation<'_>,
    allocated_host: &str,
    configured_host: Option<&str>,
    queried_server: &ServerIdentity<'_>,
) -> Result<(), AffinityError> {
    desktop(expected)?;
    if !metadata(allocated_host, 2048) {
        return Err(AffinityError::MissingExplicitSdkHost);
    }
    let configured = configured_host
        .filter(|host| metadata(host, 2048))
        .ok_or(AffinityError::MissingExplicitSdkHost)?;
    if configured != allocated_host {
        return Err(AffinityError::UnexpectedSdkHost);
    }
    same_server(&expected.server, queried_server)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn baseline() -> DesktopObservation<'static> {
        DesktopObservation {
            context: DESKTOP_CONTEXT,
            endpoint: DESKTOP_ENDPOINT,
            server: ServerIdentity {
                server_id: "private-desktop-id",
                server_version: "29.9.2",
                os_type: "linux",
            },
        }
    }
    #[test]
    fn allocated_cli_proxy_does_not_claim_sdk_observation() {
        let expected = baseline();
        let host = "unix:///var/run/docker.sock";
        let proof = verify_allocated_cli(&expected, host, Some(host), &expected.server).unwrap();
        assert_eq!(proof.summary()["sdk_connection"], false);
        assert_eq!(proof.summary()["engine_identity_matches"], true);
        assert!(verify_allocated_cli(&expected, host, None, &expected.server).is_err());
        let mut foreign = expected.server;
        foreign.server_id = "other-engine";
        assert!(verify_allocated_cli(&expected, host, Some(host), &foreign).is_err());
    }
    #[test]
    fn identical_observations_produce_only_sanitized_binding_facts() {
        let expected = baseline();
        let public = verify_desktop(&expected, &expected).unwrap().summary();
        assert_eq!(public["engine_identity_matches"], true);
        assert_eq!(public["authenticated_provenance"], false);
        assert!(!public.to_string().contains("private-desktop-id"));
    }
    #[test]
    fn different_engine_or_changed_version_requires_resolution() {
        let expected = baseline();
        let mut actual = expected;
        actual.server.server_id = "private-wsl-id";
        assert!(matches!(
            verify_desktop(&expected, &actual),
            Err(AffinityError::UnexpectedEngine)
        ));
        actual = expected;
        actual.server.server_version = "29.1.3";
        assert!(matches!(
            verify_desktop(&expected, &actual),
            Err(AffinityError::UnexpectedVersion)
        ));
    }
    #[test]
    fn context_endpoint_os_and_metadata_are_required_for_the_baseline_too() {
        let good = baseline();
        let mut bad = good;
        bad.context = "default";
        assert!(verify_desktop(&bad, &good).is_err());
        bad = good;
        bad.endpoint = "unix:///var/run/docker.sock";
        assert!(verify_desktop(&good, &bad).is_err());
        bad = good;
        bad.server.os_type = "windows";
        assert!(verify_desktop(&good, &bad).is_err());
        for id in ["", "private\nid", " id"] {
            bad = good;
            bad.server.server_id = id;
            assert!(verify_desktop(&good, &bad).is_err());
        }
    }
    #[test]
    fn sdk_needs_explicit_host_and_actual_matching_server_observation() {
        let expected = baseline();
        assert!(verify_sdk(&expected, DESKTOP_ENDPOINT, None, &expected.server).is_err());
        assert!(
            verify_sdk(
                &expected,
                DESKTOP_ENDPOINT,
                Some("unix:///var/run/docker.sock"),
                &expected.server
            )
            .is_err()
        );
        let mut foreign = expected.server;
        foreign.server_id = "private-wsl-id";
        assert!(
            verify_sdk(
                &expected,
                DESKTOP_ENDPOINT,
                Some(DESKTOP_ENDPOINT),
                &foreign
            )
            .is_err()
        );
        let proof = verify_sdk(
            &expected,
            DESKTOP_ENDPOINT,
            Some(DESKTOP_ENDPOINT),
            &expected.server,
        )
        .unwrap();
        assert_eq!(proof.summary()["sdk_connection"], true);
        assert!(
            !AffinityError::UnexpectedEngine
                .to_string()
                .contains("private")
        );
    }
}
