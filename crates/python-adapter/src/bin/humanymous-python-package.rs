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
fn inspect(wheel: &Path) -> Result<(), String> {
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
            if name.ends_with(".so") || name.ends_with(".pyd") {
                natives += 1;
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
    Ok(())
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [action] if action == "build" => build(),
        [action, wheel] if action == "inspect" => inspect(Path::new(wheel)),
        [action,wheel,python] if action=="install-test"=>install_test(Path::new(wheel),Path::new(python)),
        _ => Err("usage: humanymous-python-package build | inspect ABSOLUTE_WHEEL | install-test ABSOLUTE_WHEEL PYTHON".into()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn install_test(wheel: &Path, python: &Path) -> Result<(), String> {
    if std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref() != Ok("1") {
        return Err("clean native install/import is an explicit hosted pilot".into());
    }
    inspect(wheel)?;
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
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.python-wheel-install/1","interpreter":interpreter,
        "status":"clean-install-import-succeeded","native_api":["Wsgi","Asgi"],"generation_provenance_verified":false,
        "actual_framework_qualified":false,"whole_product_qualified":false})
    );
    Ok(())
}
