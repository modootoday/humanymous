//! Pure engine identity checks; client choice never implies daemon identity.
use serde_json::{Value, json};
pub const WINDOWS_CLIENT: &str = "C:/Program Files/Docker/Docker/resources/bin/docker.exe";
pub const DESKTOP_ENDPOINT: &str = "npipe:////./pipe/dockerDesktopLinuxEngine";
#[derive(Clone, Copy)]
pub enum Profile {
    HostedLinux,
    WindowsDesktop,
}
pub struct Expected {
    pub profile: Profile,
    pub server_id: Option<String>,
    pub version: Option<String>,
}
fn bounded(value: &Value, maximum: usize) -> Result<&str, &'static str> {
    let value = value.as_str().ok_or("engine-identity-field")?;
    if value.is_empty()
        || value.len() > maximum
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/:._+-".contains(&byte))
    {
        return Err("engine-identity-field");
    }
    Ok(value)
}
pub fn verify(expected: &Expected, context: &Value, info: &Value) -> Result<Value, &'static str> {
    let name = bounded(&context["Name"], 64)?;
    let endpoint = bounded(&context["Endpoints"]["docker"]["Host"], 256)?;
    let id = bounded(&info["ID"], 256)?;
    let version = bounded(&info["ServerVersion"], 64)?;
    if info["OSType"] != "linux" {
        return Err("engine-os-mismatch");
    }
    let (profile, client) = match expected.profile {
        Profile::WindowsDesktop => {
            if name != "desktop-linux" || endpoint != DESKTOP_ENDPOINT {
                return Err("desktop-context-endpoint-mismatch");
            }
            if expected.server_id.as_deref() != Some(id)
                || expected.version.as_deref() != Some(version)
            {
                return Err("desktop-server-identity-mismatch");
            }
            ("windows-desktop", WINDOWS_CLIENT)
        }
        Profile::HostedLinux => {
            if !endpoint.starts_with("unix:///") {
                return Err("hosted-unix-engine-required");
            }
            ("github-hosted-linux", "docker")
        }
    };
    Ok(
        json!({"profile":profile,"clientPath":client,"context":name,"endpoint":endpoint,"serverId":id,"serverVersion":version,"osType":"linux","verified":true}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Expected, Value, Value) {
        (
            Expected {
                profile: Profile::WindowsDesktop,
                server_id: Some("desktop-id".into()),
                version: Some("29.9.2".into()),
            },
            json!({"Name":"desktop-linux","Endpoints":{"docker":{"Host":DESKTOP_ENDPOINT}}}),
            json!({"ID":"desktop-id","ServerVersion":"29.9.2","OSType":"linux"}),
        )
    }
    #[test]
    fn desktop_and_wsl_native_are_not_interchangeable() {
        let (e, c, i) = fixture();
        assert!(verify(&e, &c, &i).is_ok());
        let mut wrong = c.clone();
        wrong["Endpoints"]["docker"]["Host"] = "unix:///var/run/docker.sock".into();
        assert!(verify(&e, &wrong, &i).is_err());
        let mut wrong = i.clone();
        wrong["ServerVersion"] = "29.1.3".into();
        assert!(verify(&e, &c, &wrong).is_err());
    }
    #[test]
    fn server_id_os_and_context_must_match() {
        let (e, c, i) = fixture();
        for key in ["ID", "OSType"] {
            let mut wrong = i.clone();
            wrong[key] = "other".into();
            assert!(verify(&e, &c, &wrong).is_err());
        }
        let mut wrong = c;
        wrong["Name"] = "default".into();
        assert!(verify(&e, &wrong, &i).is_err());
    }
    #[test]
    fn hosted_linux_identity_is_a_distinct_profile() {
        let expected = Expected {
            profile: Profile::HostedLinux,
            server_id: None,
            version: None,
        };
        let c =
            json!({"Name":"default","Endpoints":{"docker":{"Host":"unix:///var/run/docker.sock"}}});
        let i = json!({"ID":"hosted-id","ServerVersion":"29.1.3","OSType":"linux"});
        assert_eq!(
            verify(&expected, &c, &i).unwrap()["profile"],
            "github-hosted-linux"
        );
    }
}
