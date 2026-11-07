//! Fixed-role metadata observations; never initialize or repair custody storage.
use crate::docker::Result;
#[cfg(any(unix, test))]
use std::io::Read;
#[cfg(any(unix, test))]
fn role(name: &str) -> Result<serde_json::Value> {
    if !matches!(name, "writer" | "witness") {
        return Err("fixed writer or witness role required".into());
    }
    let contract: serde_json::Value =
        serde_json::from_str(include_str!("../../mount-probe-contract.json"))?;
    contract["roles"]
        .as_array()
        .and_then(|roles| roles.iter().find(|r| r["name"] == name))
        .cloned()
        .ok_or_else(|| "fixed writer or witness role required".into())
}
#[cfg(any(unix, test))]
fn bounded_status(input: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    input.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err("mount kernel status exceeds bound".into());
    }
    Ok(String::from_utf8(bytes)?)
}
pub fn run(name: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use crate::mount_policy::Facts;
        use serde_json::json;
        let role = role(name)?;
        let uid = u32::try_from(role["uid"].as_u64().ok_or("mount role UID")?)?;
        let status = bounded_status(std::fs::File::open("/proc/self/status")?)?;
        let field = |name: &str| {
            status
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .map(str::trim)
        };
        for name in ["Uid:", "Gid:"] {
            let values: Vec<_> = field(name)
                .ok_or("mount actor identity missing")?
                .split_whitespace()
                .collect();
            if values.len() != 4 || values.iter().any(|v| v.parse::<u32>().ok() != Some(uid)) {
                return Err("mount actor kernel identity refused".into());
            }
        }
        if field("CapEff:") != Some("0000000000000000") || field("NoNewPrivs:") != Some("1") {
            return Err("mount actor kernel hardening refused".into());
        }
        let mut paths = Vec::new();
        for path in role["paths"].as_array().ok_or("mount fixed paths")? {
            let facts = Facts::observe(std::path::Path::new(
                path["path"].as_str().ok_or("mount fixed path type")?,
            ));
            paths.push(json!({"id":path["id"],"lookup":format!("{:?}",facts.lookup),"directory":facts.directory,"symlink":facts.symlink,"uid":facts.uid,"gid":facts.gid,"mode":facts.mode,"os_error":facts.os_error,"policy_code":facts.refusal(uid).map(|v|v.code())}));
        }
        let parent_path = match name {
            "writer" => "/writer",
            "witness" => "/witness",
            _ => return Err("mount fixed parent refused".into()),
        };
        let parent = Facts::observe(std::path::Path::new(parent_path));
        let parent = json!({"lookup":format!("{:?}",parent.lookup),"directory":parent.directory,"symlink":parent.symlink,"uid":parent.uid,"gid":parent.gid,"mode":parent.mode,"os_error":parent.os_error,"meaning":"private nonpolicy observation; never a permission gate"});
        println!(
            "{}",
            json!({"schema":"humanymous.lab-mount-observation/1","role":name,"kernel_identity_verified":true,"kernel_hardening_verified":true,"paths":paths,"parent":parent,"full_product_qualified":false})
        );
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = name;
        Err("Linux in-container mount observation required".into())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn fixed_role_contract_refuses_arbitrary_role_and_path_input() {
        assert!(super::bounded_status(&[b'a'; 65537][..]).is_err());
        assert_eq!(
            super::bounded_status(&[b'a'; 65536][..]).unwrap().len(),
            65536
        );
        assert!(super::bounded_status(&[255u8][..]).is_err());
        assert_eq!(super::role("writer").unwrap()["uid"], 65532);
        assert_eq!(super::role("witness").unwrap()["uid"], 65533);
        for invalid in ["root", "/writer/keys", "writer extra", "", "../witness"] {
            assert!(super::role(invalid).is_err());
        }
    }
}
