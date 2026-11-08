//! Hosted wheel build/inspection driver. No Python executable implementation.
use pyo3::{prelude::*, types::PyTuple};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn command(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .status()
        .map_err(|_| format!("unable to launch {program}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed: {status}"))
    }
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn build() -> Result<(), String> {
    require_owned_package_lifecycle()?;
    let root = root();
    let package = root.join("adapters/python");
    if fs::read_dir(&package).map_err(|e| e.to_string())?.any(|e| {
        e.ok()
            .is_some_and(|e| e.path().extension().is_some_and(|s| s == "py"))
    }) {
        return Err("authored Python packaging implementation is forbidden".into());
    }
    let version = Command::new("maturin")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "maturin 1.16.0"
    {
        return Err("pinned maturin1.16.0 required".into());
    }
    let out = root.join("target/python-native/wheels");
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    command(
        "maturin",
        &[
            "build",
            "--locked",
            "--release",
            "--out",
            out.to_str().ok_or("output path encoding")?,
        ],
        &package,
    )
}
fn inspect(wheel: &Path) -> Result<serde_json::Value, String> {
    let wheel = wheel.canonicalize().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    fs::File::open(&wheel)
        .map_err(|e| e.to_string())?
        .take(100 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 100 * 1024 * 1024 {
        return Err("wheel size unavailable".into());
    }
    Python::initialize();
    let files = Python::attach(|py| -> PyResult<Vec<serde_json::Value>> {
        let packaging = py.import("packaging.utils")?;
        let parsed = packaging.call_method1(
            "parse_wheel_filename",
            (wheel.file_name().unwrap().to_string_lossy().as_ref(),),
        )?;
        let parsed = parsed.cast::<PyTuple>()?;
        let name: String = parsed.get_item(0)?.extract()?;
        if name != "humanymous-python-adapter" {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "unexpected distribution",
            ));
        }
        let version = parsed.get_item(1)?.str()?.to_str()?.to_owned();
        if version != "0.1.0a1" {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "unexpected wheel version",
            ));
        }
        let advertised = parsed.get_item(3)?;
        let supported = py
            .import("builtins")?
            .getattr("set")?
            .call1((py.import("packaging.tags")?.call_method0("sys_tags")?,))?;
        if advertised
            .call_method1("isdisjoint", (supported,))?
            .extract::<bool>()?
        {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "wheel is not compatible with inspection interpreter",
            ));
        }
        let archive = py
            .import("zipfile")?
            .getattr("ZipFile")?
            .call1((wheel.to_string_lossy().as_ref(),))?;
        let mut files = Vec::new();
        let mut natives = 0;
        for info in archive.call_method0("infolist")?.try_iter()? {
            let info = info?;
            let name: String = info.getattr("filename")?.extract()?;
            let size: u64 = info.getattr("file_size")?.extract()?;
            if name.starts_with('/')
                || name.contains('\\')
                || name.split('/').any(|s| s == "..")
                || size > 100 * 1024 * 1024
            {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "invalid wheel member",
                ));
            }
            if files.len() >= 128 {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "wheel member count exceeds limit",
                ));
            }
            if name.ends_with(".so") || name.ends_with(".pyd") {
                natives += 1;
                let payload: Vec<u8> = archive.call_method1("read", (&name,))?.extract()?;
                if payload.len() as u64 != size {
                    return Err(pyo3::exceptions::PyRuntimeError::new_err(
                        "native member size mismatch",
                    ));
                }
                files.push(serde_json::json!({"path":name,"size":size,
                    "sha256":hex::encode(Sha256::digest(&payload)),
                    "generation_provenance_verified":false}));
                continue;
            }
            if files.len() >= 128 {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "wheel member count exceeds limit",
                ));
            }
            files.push(serde_json::json!({"path":name,"size":size,
                "generation_provenance_verified":false}));
        }
        archive.call_method0("close")?;
        if natives != 1 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "exactly one native extension required",
            ));
        }
        Ok(files)
    })
    .map_err(|e| e.to_string())?;
    let receipt = serde_json::json!({"schema":"humanymous.python-wheel-inspection/1","wheel":wheel.file_name().unwrap().to_string_lossy(),
        "wheel_sha256":hex::encode(Sha256::digest(bytes)),"expected_generator":{"name":"maturin","version":"1.16.0"},
        "generation_provenance_verified":false,
        "files":files,"actual_framework_qualified":false,"whole_product_qualified":false,"status":"archive-inspected-only"});
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|e| e.to_string())?
    );
    Ok(receipt)
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [action] if action == "build" => build(),
        [action, wheel] if action == "inspect" => inspect(Path::new(wheel)).map(|_|()),
        [action,wheel,python] if action=="install-test"=>install_test(Path::new(wheel),Path::new(python)),
        _ => Err("usage: humanymous-python-package build | inspect ABSOLUTE_WHEEL | install-test ABSOLUTE_WHEEL PYTHON".into()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn install_test(wheel: &Path, python: &Path) -> Result<(), String> {
    require_owned_package_lifecycle()?;
    if std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref() != Ok("1") {
        return Err("clean native install/import is an explicit hosted pilot".into());
    }
    let archive_receipt = inspect(wheel)?;
    let wheel = wheel.canonicalize().map_err(|e| e.to_string())?;
    let version = Command::new(python)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if !version.status.success() {
        return Err("inspection interpreter unavailable".into());
    }
    let interpreter = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    let expected = Python::attach(|py| -> PyResult<String> {
        let info = py.import("sys")?.getattr("version_info")?;
        Ok(format!(
            "Python {}.{}.",
            info.get_item(0)?.extract::<u8>()?,
            info.get_item(1)?.extract::<u8>()?
        ))
    })
    .map_err(|e| e.to_string())?;
    if !interpreter.starts_with(&expected) {
        return Err("pip and inspection interpreter versions differ".into());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = root()
        .join("target/python-native/install")
        .join(format!("{}-{stamp}", std::process::id()));
    let parent = directory.parent().unwrap();
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    fs::create_dir(&directory).map_err(|e| e.to_string())?;
    let status = Command::new(python)
        .args([
            "-m",
            "pip",
            "install",
            "--no-deps",
            "--no-index",
            "--target",
        ])
        .arg(&directory)
        .arg(&wheel)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("clean wheel installation failed: {status}"));
    }
    Python::attach(|py| -> PyResult<()> {
        let sys = py.import("sys")?;
        if sys.getattr("modules")?.contains("humanymous_python")? {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "native module already imported before clean install",
            ));
        }
        sys.getattr("path")?
            .call_method1("insert", (0, directory.to_string_lossy().as_ref()))?;
        let module = py.import("humanymous_python")?;
        let capabilities: String = module.call_method0("capabilities")?.extract()?;
        let capabilities: serde_json::Value =
            serde_json::from_str(&capabilities).map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("invalid installed capability receipt")
            })?;
        validate_role(&capabilities, "python", "production")
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        if capabilities["adapter"] != "humanymous.python"
            || capabilities["hostPilotEnabled"] != false
            || capabilities["packageToolEnabled"] != false
        {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "fixture or package tool present in production wheel",
            ));
        }
        let file: PathBuf = module.getattr("__file__")?.extract()?;
        let actual = file.canonicalize().map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err("installed module path unavailable")
        })?;
        let expected = directory.canonicalize().map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err("install directory unavailable")
        })?;
        if !actual.starts_with(&expected)
            || !module.getattr("Wsgi")?.is_callable()
            || !module.getattr("Asgi")?.is_callable()
        {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "installed native API unavailable",
            ));
        }
        let relative = actual.strip_prefix(&expected).map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err("installed path outside root")
        })?;
        let relative = relative
            .to_str()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("installed path encoding"))?
            .replace('\\', "/");
        let mut payload = Vec::new();
        fs::File::open(&actual)
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("installed native bytes unavailable")
            })?
            .take(100 * 1024 * 1024 + 1)
            .read_to_end(&mut payload)
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("installed native bytes unreadable")
            })?;
        if payload.is_empty() || payload.len() > 100 * 1024 * 1024 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "installed native size exceeds limit",
            ));
        }
        join_native_member(&archive_receipt, &relative, &payload)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.python-wheel-install/1","interpreter":interpreter,
        "status":"clean-install-import-succeeded","native_artifact_join_verified":true,"native_api":["Wsgi","Asgi"],"generation_provenance_verified":false,
        "actual_framework_qualified":false,"whole_product_qualified":false})
    );
    Ok(())
}

fn validate_role(value: &serde_json::Value, context: &str, role: &str) -> Result<(), String> {
    let expected_flags = match role {
        "production" => (false, false),
        "host-pilot" => (true, false),
        "package-tool" => (false, true),
        _ => return Err("unsupported artifact role".into()),
    };
    let metadata = &value["artifactRole"];
    if value["adapter"] != format!("humanymous.{context}")
        || metadata["schema"] != "humanymous.host-artifact-role/1"
        || metadata["context"] != context
        || metadata["abiMajor"].as_u64() != Some(1)
        || metadata["role"] != role
        || value["hostPilotEnabled"].as_bool() != Some(expected_flags.0)
        || value["packageToolEnabled"].as_bool() != Some(expected_flags.1)
    {
        return Err("artifact context, role, version or features do not match".into());
    }
    Ok(())
}

#[cfg(test)]
mod role_tests {
    use super::validate_role;
    use serde_json::json;
    #[test]
    fn rejects_shared_package_identity_without_exact_role_metadata() {
        for context in ["python", "ruby"] {
            let valid = json!({"adapter":format!("humanymous.{context}"),
                "artifactRole":{"schema":"humanymous.host-artifact-role/1",
                "context":context,"abiMajor":1,"role":"production"},
                "hostPilotEnabled":false,"packageToolEnabled":false});
            assert!(validate_role(&valid, context, "production").is_ok());
            for (pointer, replacement) in [
                ("/artifactRole/role", json!("host-pilot")),
                ("/artifactRole/abiMajor", json!(2)),
                (
                    "/artifactRole/schema",
                    json!("humanymous.host-artifact-role/2"),
                ),
                ("/artifactRole/context", json!("node")),
                ("/hostPilotEnabled", json!(true)),
                ("/packageToolEnabled", json!(true)),
                ("/hostPilotEnabled", serde_json::Value::Null),
            ] {
                let mut wrong = valid.clone();
                *wrong.pointer_mut(pointer).unwrap() = replacement;
                assert!(
                    validate_role(&wrong, context, "production").is_err(),
                    "{pointer}"
                );
            }
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove("artifactRole");
            assert!(validate_role(&missing, context, "production").is_err());
            assert!(validate_role(&valid, context, "package-tool").is_err());
            assert!(validate_role(&valid, context, "unsupported-combined").is_err());
        }
    }
}

fn join_native_member(
    receipt: &serde_json::Value,
    installed_relative: &str,
    payload: &[u8],
) -> Result<(), String> {
    let members = receipt["files"]
        .as_array()
        .ok_or("wheel member manifest missing")?;
    let natives: Vec<_> = members
        .iter()
        .filter(|row| {
            row["path"]
                .as_str()
                .is_some_and(|name| name.ends_with(".so") || name.ends_with(".pyd"))
        })
        .collect();
    let [native] = natives.as_slice() else {
        return Err("exactly one wheel native member required".into());
    };
    if receipt["schema"] != "humanymous.python-wheel-inspection/1"
        || native["path"].as_str() != Some(installed_relative)
        || native["size"].as_u64() != Some(payload.len() as u64)
        || native["sha256"] != hex::encode(Sha256::digest(payload))
    {
        return Err("installed native path or bytes do not match inspected wheel".into());
    }
    Ok(())
}

#[cfg(test)]
mod join_tests {
    use super::*;
    #[test]
    fn imported_bytes_must_join_exactly_one_archive_member() {
        let bytes = b"owned native fixture";
        let row = serde_json::json!({"path":"humanymous_python.so","size":bytes.len(),"sha256":hex::encode(Sha256::digest(bytes))});
        let valid = serde_json::json!({"schema":"humanymous.python-wheel-inspection/1","files":[row.clone()]});
        assert!(join_native_member(&valid, "humanymous_python.so", bytes).is_ok());
        assert!(join_native_member(&valid, "other/humanymous_python.so", bytes).is_err());
        assert!(join_native_member(&valid, "humanymous_python.so", b"replacement bytes").is_err());
        for rows in [serde_json::json!([]), serde_json::json!([row.clone(), row])] {
            let mut invalid = valid.clone();
            invalid["files"] = rows;
            assert!(join_native_member(&invalid, "humanymous_python.so", bytes).is_err());
        }
        let mut invalid = valid.clone();
        invalid["files"][0]["sha256"] = serde_json::Value::Null;
        assert!(join_native_member(&invalid, "humanymous_python.so", bytes).is_err());
    }
}

fn require_owned_package_lifecycle() -> Result<(), String> {
    Err(
        "NOT_ACCEPTED: package execution requires an allocated shared owned-process lifecycle"
            .into(),
    )
}

#[cfg(test)]
mod lifecycle_tests {
    #[test]
    fn unqualified_package_lifecycle_cannot_authorize_spawn_or_cleanup() {
        assert!(
            super::require_owned_package_lifecycle()
                .unwrap_err()
                .starts_with("NOT_ACCEPTED:")
        );
    }
}
