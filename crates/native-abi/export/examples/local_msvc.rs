//! Explicit local Windows header compilation and passive DLL inspection only.
#![forbid(unsafe_code)]

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Local MSVC artifact verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn absolute_file(value: OsString) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_file() {
        return Err("absolute existing input file required".into());
    }
    Ok(path)
}

fn invoke(tool: &Path, args: &[OsString], out: &Path, label: &str) -> Result<(), String> {
    let result = Command::new(tool)
        .args(args)
        .current_dir(out)
        .output()
        .map_err(|error| format!("{label} spawn: {error}"))?;
    fs::write(out.join(format!("{label}-stdout.txt")), &result.stdout)
        .map_err(|error| error.to_string())?;
    fs::write(out.join(format!("{label}-stderr.txt")), &result.stderr)
        .map_err(|error| error.to_string())?;
    fs::write(
        out.join(format!("{label}-status.txt")),
        format!("{}\n", result.status),
    )
    .map_err(|error| error.to_string())?;
    if !result.status.success() {
        return Err(format!("{label} exited {}", result.status));
    }
    Ok(())
}

fn run() -> Result<(), String> {
    if !cfg!(all(target_os = "windows", target_arch = "x86_64"))
        || std::env::var_os("GITHUB_ACTIONS").is_some()
        || std::env::var_os("ACT").is_some()
    {
        return Err("requires actual local Windows x64 outside GitHub/Act".into());
    }
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--local-msvc")) {
        return Err("explicit --local-msvc profile required".into());
    }
    let values: Vec<_> = args.collect();
    if values.len() != 5 {
        return Err("expected CL DUMPBIN GENERATED_HEADER DLL FRESH_OUTPUT_DIR".into());
    }
    let cl = absolute_file(values[0].clone())?;
    let dumpbin = absolute_file(values[1].clone())?;
    let header = absolute_file(values[2].clone())?;
    let dll = absolute_file(values[3].clone())?;
    let out = PathBuf::from(&values[4]);
    if !out.is_absolute() {
        return Err("absolute fresh output directory required".into());
    }
    let bytes = fs::read(&header).map_err(|error| error.to_string())?;
    if bytes.len() > 131_072
        || !std::str::from_utf8(&bytes)
            .map_err(|error| error.to_string())?
            .contains("humanymous_native_abi_v1(")
    {
        return Err("bounded generated ABI header required".into());
    }
    fs::create_dir(&out).map_err(|error| format!("fresh output: {error}"))?;
    for (label, language, standard) in [("c11", "/TC", "/std:c11"), ("cpp14", "/TP", "/std:c++14")]
    {
        let object = out.join(format!("{label}.obj"));
        let mut object_flag = OsString::from("/Fo");
        object_flag.push(&object);
        invoke(
            &cl,
            &[
                "/nologo".into(),
                "/c".into(),
                language.into(),
                standard.into(),
                "/W4".into(),
                "/WX".into(),
                object_flag,
                header.clone().into_os_string(),
            ],
            &out,
            label,
        )?;
        if !object.is_file()
            || fs::metadata(&object)
                .map_err(|error| error.to_string())?
                .len()
                == 0
        {
            return Err(format!("{label} did not produce an object"));
        }
    }
    invoke(
        &dumpbin,
        &["/headers".into(), "/exports".into(), dll.into_os_string()],
        &out,
        "dll-inspection",
    )?;
    fs::write(out.join("scope.txt"), "Local Windows x64 MSVC C11 and C++14 generated-header compilation passed. DLL inspection is passive. Layout equivalence, C++11, dynamic calls, C/C++ callers, RAII, exceptions and product e2e: not_run.\n")
        .map_err(|error| error.to_string())?;
    println!("Local MSVC C11/C++14 header compilation passed; actual dynamic ABI not_run");
    Ok(())
}
