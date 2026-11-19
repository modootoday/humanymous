//! `carrier-owner-fixture`: the foreign-owner case of D5-N4 option (a)
//! (issue #75 comment 6999924070), decided as (ii) strengthened.
//!
//! The fixture measures `SeRestorePrivilege` and `SeTakeOwnershipPrivilege`
//! in the actual token. With neither enabled, a file with owner Everyone
//! (`O:WD`) must be refused at creation. With either enabled, that creation
//! must succeed (the observed fact), and then the real guard must hold: both
//! publishers refuse an Everyone-owned carrier folder or file with the
//! existing "protected CI actor carrier required" refusal and clean up what
//! they wrote. Neither branch is a skip; every case asserts.
use super::{
    account, actor_for_nonce,
    prebuilt_run::{OwnerAtCreation, native, wire},
};
use crate::evidence;
use humanymous_platform::windows_object::{
    create_directory_with_descriptor, create_file::create_new_file_with_descriptor,
};
use humanymous_runtime::native_security::{
    process::{current_identity, current_owner_privileges},
    storage::{
        CarrierCreator, ContextPublishError, CreatedCarrierFile, PublishedActorContext,
        publish_actor_context, publish_actor_context_created_with,
    },
};
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};
use windows_permissions::{
    LocalBox, SecurityDescriptor,
    constants::{SeObjectType, SecurityInformation},
    wrappers,
};

pub(crate) const MODE: &str = "carrier-owner-fixture";
const SCHEMA: &str = "humanymous.carrier-owner-fixture/1";
const REFUSAL: &str = "protected CI actor carrier required";
const EVERYONE: &str = "S-1-1-0";

fn other(reason: &'static str) -> io::Error {
    io::Error::other(reason)
}

#[derive(Clone, Copy)]
enum Foreign {
    Directory,
    ContextFile,
}
/// The publisher's own creator with `O:BA` swapped for `O:WD` on one object.
struct ForeignOwner(Foreign);
fn everyone(sddl: &str) -> io::Result<String> {
    sddl.strip_prefix("O:BA")
        .map(|rest| format!("O:WD{rest}"))
        .ok_or_else(|| other("publisher descriptor without O:BA"))
}
impl CarrierCreator for ForeignOwner {
    fn create_directory(&self, path: &Path, sddl: &str) -> io::Result<()> {
        match self.0 {
            Foreign::Directory => OwnerAtCreation.create_directory(path, &everyone(sddl)?),
            Foreign::ContextFile => OwnerAtCreation.create_directory(path, sddl),
        }
    }
    fn create_file(&self, path: &Path, sddl: &str) -> io::Result<Box<dyn CreatedCarrierFile>> {
        let context = path.file_name() == Some(OsStr::new("context.json"));
        match self.0 {
            Foreign::ContextFile if context => OwnerAtCreation.create_file(path, &everyone(sddl)?),
            _ => OwnerAtCreation.create_file(path, sddl),
        }
    }
}

fn owner_of(path: &Path) -> io::Result<String> {
    let sd = wrappers::GetNamedSecurityInfo(
        path.as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner,
    )?;
    sd.owner()
        .map(|sid| sid.to_string())
        .ok_or_else(|| other("owner missing"))
}

/// Outcome of one publisher call, with the publisher's own cleanup applied.
fn settle(result: Result<PublishedActorContext, ContextPublishError>, root: &Path) -> Value {
    let (outcome, error, cleaned) = match result {
        Ok(carrier) => ("published", None, carrier.finish().is_ok()),
        Err(ContextPublishError::BeforeCreation(error)) => {
            ("refused-before-creation", Some(error.to_string()), true)
        }
        Err(ContextPublishError::Retained { owner, error }) => (
            "refused-after-creation",
            Some(error.to_string()),
            owner.finish().is_ok(),
        ),
    };
    json!({
        "outcome": outcome,
        "error": error,
        "publisher_cleanup_ok": cleaned,
        "carrier_absent": !root.exists(),
    })
}
fn refused_and_clean(case: &Value) -> bool {
    case["outcome"] == "refused-after-creation"
        && case["error"] == REFUSAL
        && case["publisher_cleanup_ok"] == true
        && case["carrier_absent"] == true
}

/// Elevated hosted parent: `carrier-owner-fixture --report <abs>`.
pub(crate) fn run(report: &Path) -> io::Result<i32> {
    if !report.is_absolute() {
        return Err(other("absolute --report required"));
    }
    if !evidence::disposable_host() {
        return Err(other("disposable GitHub-hosted actor required"));
    }
    let identity = current_identity()?;
    if !identity.elevated() || !identity.enabled_administrator() {
        return Err(other("hosted provisioning administrator required"));
    }
    let privileges = current_owner_privileges()?;
    let privileged = privileges.restore || privileges.take_ownership;
    let mut facts = json!({
        "token_privileges": {
            "SeRestorePrivilege_enabled": privileges.restore,
            "SeTakeOwnershipPrivilege_enabled": privileges.take_ownership,
        },
        "branch": if privileged { "foreign-owner-assignable" } else { "foreign-owner-refused" },
        "runner_image": std::env::var("ImageVersion").ok(),
    });
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(io::Error::other)?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let (name, password) = actor_for_nonce(&nonce)?;
    let mut owner = account(&name, password)?;
    let (sid, _, _) = wrappers::LookupAccountName(Option::<&OsStr>::None, &name)?;
    let sid = sid.to_string();
    let base =
        PathBuf::from(std::env::var_os("ProgramData").ok_or_else(|| other("ProgramData missing"))?)
            .canonicalize()?;
    let base = native(&wire(&base)?);
    let probe_name = format!("hmown-{nonce}");
    let probe = base.join(&probe_name);
    let root = base.join(format!("hmctx-{nonce}"));
    let (context, binding) = (
        b"{\"fixture\":\"carrier-owner\"}".as_slice(),
        b"{}".as_slice(),
    );
    let mut failures: Vec<String> = Vec::new();
    let outcome = (|| -> io::Result<()> {
        let sd: LocalBox<SecurityDescriptor> = "O:BAD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)".parse()?;
        let _probe = create_directory_with_descriptor(&probe, &sd)
            .map_err(|_| other("probe directory owner-at-creation refused"))?;

        // 1. Raw creation with owner Everyone, branch by measured privileges.
        let foreign_path = probe.join("foreign.json");
        let foreign: LocalBox<SecurityDescriptor> = "O:WDD:P(A;;FA;;;BA)(A;;FA;;;SY)".parse()?;
        let created = create_new_file_with_descriptor(&foreign_path, &foreign);
        let accepted = created.is_ok();
        drop(created);
        let owner_seen = if accepted {
            owner_of(&foreign_path).ok()
        } else {
            None
        };
        facts["foreign_owner_create"] = json!({
            "accepted": accepted,
            "owner": owner_seen,
            "exists_after": foreign_path.exists(),
        });
        if privileged {
            if !(accepted && owner_seen.as_deref() == Some(EVERYONE)) {
                failures.push("privileged token: O:WD creation not observed".into());
            }
        } else if accepted || foreign_path.exists() {
            failures.push("unprivileged token: O:WD creation was not refused".into());
        }
        if foreign_path.exists() {
            fs::remove_file(&foreign_path)?;
        }

        // 2. publish_actor_context_created_with, Everyone-owned folder, then file.
        for (label, kind) in [
            ("created_with_foreign_directory", Foreign::Directory),
            ("created_with_foreign_context_file", Foreign::ContextFile),
        ] {
            let case = settle(
                publish_actor_context_created_with(
                    &ForeignOwner(kind),
                    &nonce,
                    &sid,
                    context,
                    binding,
                ),
                &root,
            );
            let ok = if privileged {
                refused_and_clean(&case)
            } else {
                // Creation itself is refused; nothing may remain either way.
                case["outcome"] != "published"
                    && case["publisher_cleanup_ok"] == true
                    && case["carrier_absent"] == true
            };
            if !ok {
                failures.push(format!("{label}: publisher did not refuse and clean up"));
            }
            facts[label] = case;
        }

        // 3. publish_actor_context: its create-new path never adopts an
        // existing object, so an Everyone-owned folder at the carrier path is
        // refused before creation and left to its creator.
        if privileged {
            let squat: LocalBox<SecurityDescriptor> =
                "O:WDD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)".parse()?;
            let squatted = create_directory_with_descriptor(&root, &squat).is_ok();
            let case = settle(
                publish_actor_context(&nonce, &sid, context, binding),
                &PathBuf::from("\\nonexistent"),
            );
            let untouched = owner_of(&root).ok().as_deref() == Some(EVERYONE);
            facts["plain_squatted_everyone_directory"] = json!({
                "squatted": squatted,
                "case": case,
                "squat_untouched": untouched,
            });
            if !(squatted && case["outcome"] == "refused-before-creation" && untouched) {
                failures.push("plain publisher adopted or altered an Everyone-owned folder".into());
            }
            if root.exists() {
                fs::remove_dir(&root)?;
            }
        }
        // 4. publish_actor_context with the token default owner, as on the
        // N4 runs: anything but Administrators/SYSTEM is the existing refusal.
        let case = settle(publish_actor_context(&nonce, &sid, context, binding), &root);
        let ok = refused_and_clean(&case)
            || (case["outcome"] == "published"
                && case["publisher_cleanup_ok"] == true
                && case["carrier_absent"] == true);
        if !ok {
            failures
                .push("plain publisher: neither the existing refusal nor a clean publish".into());
        }
        facts["plain_default_owner"] = case;
        Ok(())
    })();
    if let Err(error) = &outcome {
        failures.push(format!("fixture: {error}"));
    }
    if root.exists() && fs::remove_dir_all(&root).is_err() {
        failures.push("carrier root residue".into());
    }
    let probe_removed = probe.parent() == Some(base.as_path())
        && probe.file_name() == Some(OsStr::new(&probe_name))
        && match fs::remove_dir_all(&probe) {
            Err(error) => error.kind() == io::ErrorKind::NotFound,
            Ok(()) => true,
        };
    if !probe_removed {
        failures.push("probe directory residue".into());
    }
    if owner.delete_legacy_account().is_err() {
        failures.push("account deletion".into());
    }
    let passed = failures.is_empty();
    let value = json!({
        "schema": SCHEMA,
        "sot": "sot:root/dotnet-protected-host-launch N4",
        "decision": "D5-N4 option (a) issue 75 comment 6999924070; foreign-owner case (ii) strengthened",
        "status": if passed { "passed" } else { "failed" },
        "facts": facts,
        "failures": failures,
    });
    evidence::write(report, &value)?;
    Ok(if passed { 0 } else { 1 })
}
