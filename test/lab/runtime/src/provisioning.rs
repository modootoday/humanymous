//! Actual CLI process lifecycle against synthetic owned artifacts in Docker.
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

fn invoke(root: &Path, digest: &str, action: &str, dry: bool) -> Output {
    let mut command = Command::new("/usr/local/bin/humanymous");
    command
        .arg(action)
        .arg("--artifact")
        .arg(root.join("source"))
        .arg("--destination")
        .arg(root.join("installed"))
        .args(["--sha256", digest, "--local-management-verified"]);
    if dry {
        command.arg("--dry-run");
    }
    command.output().unwrap()
}
fn names(root: &Path) -> Vec<std::ffi::OsString> {
    let mut names = fs::read_dir(root)
        .unwrap()
        .map(|r| r.unwrap().file_name())
        .collect::<Vec<_>>();
    names.sort();
    names
}
pub fn run(root: &Path) {
    let root = root.join("cli");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let artifact = b"synthetic non-executed provisioning artifact";
    fs::write(root.join("source"), artifact).unwrap();
    let digest = format!("{:x}", Sha256::digest(artifact));
    let before = names(&root);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
    let refused = invoke(&root, &digest, "doctor", false);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        !refused.status.success(),
        "doctor accepted an unwritable parent"
    );
    for (action, dry) in [("doctor", false), ("install", true)] {
        let output = invoke(&root, &digest, action, dry);
        assert!(output.status.success(), "read-only CLI refused");
        assert_eq!(names(&root), before);
        assert_eq!(fs::read(root.join("source")).unwrap(), artifact);
    }
    for (expected, action) in [("changed=true", "install"), ("changed=false", "install")] {
        let output = invoke(&root, &digest, action, false);
        assert!(output.status.success(), "CLI installation refused");
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
    assert!(invoke(&root, &digest, "rollback", false).status.success());
    assert_eq!(names(&root), before);
    assert!(
        !invoke(&root, &"0".repeat(64), "doctor", false)
            .status
            .success()
    );
    assert_eq!(names(&root), before);
    assert!(invoke(&root, &digest, "install", false).status.success());
    fs::write(root.join("installed/artifact"), b"tampered").unwrap();
    assert!(!invoke(&root, &digest, "rollback", false).status.success());
    assert_eq!(
        fs::read(root.join("installed/artifact")).unwrap(),
        b"tampered"
    );
    // Remove only the synthetic files this driver created, never a user tree.
    for name in ["artifact", "receipt"] {
        fs::remove_file(root.join("installed").join(name)).unwrap();
    }
    fs::remove_dir(root.join("installed")).unwrap();
}
