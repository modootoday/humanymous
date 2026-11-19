//! Actual CLI observations are private; shared test-support owns comparisons.
use super::{CLOUD_CONTAINER_ENDPOINT, Docker, Result, Venue, actual_hosted_linux};
use humanymous_platform::testing::{
    docker::{DesktopObservation, ServerIdentity, verify_allocated_cli, verify_desktop},
    json::decode_strict_json,
    process::capture_with_stderr,
};
use serde_json::{Value, json};
use std::{fs, time::Duration};

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    Ok(value[name]
        .as_str()
        .ok_or("Docker engine metadata field missing")?)
}
fn baseline(value: &Value) -> Result<DesktopObservation<'_>> {
    let keys = [
        "schema",
        "context",
        "endpoint",
        "server_id",
        "server_version",
        "os_type",
    ];
    if value["schema"] != "humanymous.lab-desktop-engine-baseline/1"
        || !value
            .as_object()
            .is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
    {
        return Err("Desktop baseline schema/fields refused".into());
    }
    let observed = DesktopObservation {
        context: field(value, "context")?,
        endpoint: field(value, "endpoint")?,
        server: ServerIdentity {
            server_id: field(value, "server_id")?,
            server_version: field(value, "server_version")?,
            os_type: field(value, "os_type")?,
        },
    };
    // Validate the independently supplied baseline before observing any daemon.
    verify_desktop(&observed, &observed)?;
    Ok(observed)
}
/// Pure judgement for the explicit cloud-container venue: the observed engine
/// must be the Linux engine whose ID the operator supplied before the run.
fn cloud_container_proof(expected: &str, server: &ServerIdentity<'_>) -> Result<Value> {
    if server.server_id.is_empty()
        || server.server_id != expected
        || server.server_version.is_empty()
        || server.server_version.len() > 128
        || server.os_type != "linux"
    {
        return Err(
            "cloud-container engine is not the Linux engine whose ID was supplied; refuse before mutation"
                .into(),
        );
    }
    Ok(
        json!({"qualification":"explicit-cloud-container-engine-observation","expected_engine_id":expected,
        "engine_id_matched":true,"authenticated_provenance":false,"qualifying_venue":false,
        "desktop_equivalence_claimed":false,"github_hosted":false}),
    )
}
impl Docker {
    /// Explicit cloud-container preflight: the endpoint is the fixed local
    /// socket passed with `--host`, never the selected context.
    fn cloud_container(&mut self, expected: &str) -> Result<()> {
        let bytes = self.engine_capture(&["info", "--format", "{{json .}}"])?;
        let info = decode_strict_json(&bytes, 1048576)?;
        let server = ServerIdentity {
            server_id: field(&info, "ID")?,
            server_version: field(&info, "ServerVersion")?,
            os_type: field(&info, "OSType")?,
        };
        let proof = cloud_container_proof(expected, &server)?;
        let observed = json!({"context":null,"endpoint":CLOUD_CONTAINER_ENDPOINT,
            "endpoint_selection":"explicit --host","server_id":server.server_id,
            "server_version":server.server_version,"os_type":server.os_type});
        if self
            .engine_identity
            .as_ref()
            .is_some_and(|previous| previous["observed"] != observed)
        {
            return Err(
                "Docker engine identity changed during lab; resolve before mutation or cleanup"
                    .into(),
            );
        }
        self.engine_identity = Some(
            json!({"schema":"humanymous.lab-engine-observation/1","venue":"cloud-container","observed":observed,
            "desktop_baseline":null,"proof":proof,"sdk_affinity_claimed":false,
            "credential_helper_presence_checked":false,"credential_contents_read":false}),
        );
        Ok(())
    }
    fn engine_capture(&mut self, args: &[&str]) -> Result<Vec<u8>> {
        let args: Vec<String> = args.iter().map(|s| (*s).into()).collect();
        let mut command = self.raw_command(&args, &[])?;
        let output = capture_with_stderr(&mut command, &[], Duration::from_secs(20), 1048576)?;
        self.transcript.push(json!({"phase":"engine_preflight","argv":args,"exit":output.status.code(),"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr)}));
        if !output.status.success() {
            return Err("Docker engine preflight command failed; see private transcript".into());
        }
        Ok(output.stdout)
    }
    pub(super) fn preflight(&mut self, force: bool) -> Result<()> {
        if self.engine_identity.is_some() && !force {
            return Ok(());
        }
        if let Venue::CloudContainer { engine_id } = &self.venue {
            let expected = engine_id.clone();
            return self.cloud_container(&expected);
        }
        let hosted = actual_hosted_linux();
        if !hosted && self.desktop_baseline.is_none() {
            let path = std::env::var_os("HUMANYMOUS_DOCKER_DESKTOP_BASELINE").ok_or(
                "independently observed Desktop baseline file required before local Docker",
            )?;
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 16384 {
                return Err("Desktop baseline file type/size refused".into());
            }
            let value = decode_strict_json(&fs::read(&path)?, 16384)?;
            baseline(&value)?;
            self.desktop_baseline = Some(value);
        }
        if cfg!(windows) {
            let parent = self
                .executable
                .parent()
                .ok_or("Windows Docker tool directory")?;
            if !parent.join("docker-credential-desktop.exe").is_file() {
                return Err("Windows Desktop credential helper executable missing; no fallback or credential-store edit".into());
            }
        }
        let context = if cfg!(windows) {
            "desktop-linux".to_owned()
        } else {
            String::from_utf8(self.engine_capture(&["context", "show"])?)?
                .trim()
                .to_owned()
        };
        if context.is_empty()
            || context.len() > 128
            || !context
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err("Docker selected context name refused".into());
        }
        let endpoint = if !cfg!(windows) && !hosted {
            std::env::var("HUMANYMOUS_DOCKER_DESKTOP_PROXY")
                .map_err(|_| "explicit allocated Desktop proxy absent")?
        } else if hosted
            && std::env::var("DOCKER_HOST")
                .ok()
                .is_some_and(|s| !s.is_empty())
        {
            std::env::var("DOCKER_HOST")?
        } else {
            let bytes = self.engine_capture(&[
                "context",
                "inspect",
                &context,
                "--format",
                "{{json .Endpoints.docker.Host}}",
            ])?;
            decode_strict_json(&bytes, 16384)?
                .as_str()
                .ok_or("Docker context endpoint absent")?
                .to_owned()
        };
        let bytes = self.engine_capture(&["info", "--format", "{{json .}}"])?;
        let info = decode_strict_json(&bytes, 1048576)?;
        let server = ServerIdentity {
            server_id: field(&info, "ID")?,
            server_version: field(&info, "ServerVersion")?,
            os_type: field(&info, "OSType")?,
        };
        let proof = if hosted {
            if server.server_id.is_empty()
                || server.server_id.len() > 256
                || server.server_version.is_empty()
                || server.server_version.len() > 128
                || server.os_type != "linux"
                || endpoint.is_empty()
            {
                return Err("hosted Linux engine identity incomplete".into());
            }
            json!({"qualification":"assigned-hosted-engine-observation","authenticated_provenance":false,"desktop_equivalence_claimed":false})
        } else {
            let expected = baseline(
                self.desktop_baseline
                    .as_ref()
                    .ok_or("Desktop baseline missing")?,
            )?;
            if cfg!(windows) {
                verify_desktop(
                    &expected,
                    &DesktopObservation {
                        context: &context,
                        endpoint: &endpoint,
                        server,
                    },
                )?
                .summary()
            } else {
                verify_allocated_cli(
                    &expected,
                    &endpoint,
                    std::env::var("DOCKER_HOST").ok().as_deref(),
                    &server,
                )?
                .summary()
            }
        };
        let observed = json!({"context":context,"endpoint":endpoint,"server_id":server.server_id,"server_version":server.server_version,"os_type":server.os_type});
        if self
            .engine_identity
            .as_ref()
            .is_some_and(|previous| previous["observed"] != observed)
        {
            return Err("Docker engine/context identity changed during lab; resolve before mutation or cleanup".into());
        }
        self.engine_identity = Some(
            json!({"schema":"humanymous.lab-engine-observation/1","venue":if hosted{"github-hosted-linux"}else if cfg!(windows){"local-windows-desktop"}else{"allocated-desktop-cli-proxy"},"observed":observed,"desktop_baseline":self.desktop_baseline,"proof":proof,"sdk_affinity_claimed":false,"credential_helper_presence_checked":cfg!(windows),"credential_contents_read":false}),
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn cloud_container_binds_the_supplied_engine_id_and_never_qualifies() {
        use super::{ServerIdentity, cloud_container_proof};
        let server = |id, os| ServerIdentity {
            server_id: id,
            server_version: "29.1.3",
            os_type: os,
        };
        let proof = cloud_container_proof("engine-a", &server("engine-a", "linux")).unwrap();
        assert_eq!(proof["expected_engine_id"], "engine-a");
        assert_eq!(proof["authenticated_provenance"], false);
        assert_eq!(proof["qualifying_venue"], false);
        for observed in [
            server("engine-b", "linux"),
            server("", "linux"),
            server("engine-a", "windows"),
        ] {
            assert!(cloud_container_proof("engine-a", &observed).is_err());
        }
        assert!(cloud_container_proof("", &server("", "linux")).is_err());
    }
    #[test]
    fn baseline_is_explicit_strict_and_never_inferred_from_current_daemon() {
        use serde_json::json;
        let value = json!({"schema":"humanymous.lab-desktop-engine-baseline/1","context":"desktop-linux","endpoint":"npipe:////./pipe/dockerDesktopLinuxEngine","server_id":"owned-observed-id","server_version":"29.9.2","os_type":"linux"});
        assert!(super::baseline(&value).is_ok());
        for (key, v) in [
            ("context", json!("default")),
            ("server_id", json!("")),
            ("endpoint", json!("unix:///var/run/docker.sock")),
            ("os_type", json!("windows")),
            ("unknown", json!(true)),
        ] {
            let mut wrong = value.clone();
            wrong[key] = v;
            assert!(super::baseline(&wrong).is_err());
        }
    }
}
