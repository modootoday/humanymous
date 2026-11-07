//! Read-only Windows tool discovery. Presence is not runtime qualification.
use serde::Serialize;
use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};
mod probe;

#[derive(Debug, Serialize)]
pub struct Tool {
    pub name: &'static str,
    pub path: Option<PathBuf>,
    pub runtime_validated: bool,
    pub version: Option<probe::Version>,
}

#[derive(Debug, Serialize)]
pub struct SdkFile {
    pub name: &'static str,
    pub path: Option<PathBuf>,
    pub link_validated: bool,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub windows_host: bool,
    pub tools: Vec<Tool>,
    pub child_environment: Vec<(&'static str, String)>,
    pub msvc_setup: Option<PathBuf>,
    pub msvc_setup_arguments: [&'static str; 2],
    pub linux_container_arguments: [&'static str; 2],
    pub python_sdk: Vec<SdkFile>,
    pub ruby_extension_abi_qualified: bool,
    pub complete: bool,
    pub notes: Vec<&'static str>,
}

fn first_file(paths: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    paths
        .into_iter()
        .find(|path| path.is_absolute() && path.is_file())
}

fn candidates(name: &str, extra: Vec<PathBuf>) -> Option<PathBuf> {
    first_file(
        extra.into_iter().chain(
            env::var_os("PATH")
                .into_iter()
                .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
                .map(|dir| dir.join(name)),
        ),
    )
}

/// Never installs, launches a shell, mutates PATH, or initializes a model/container.
pub fn inspect(probe_versions: bool) -> Report {
    let start = Instant::now();
    let local = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    let user = env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_default();
    let program = env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_default();
    let program_x86 = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_default();
    let vs_roots: Vec<_> = [&program_x86, &program]
        .into_iter()
        .flat_map(|base| {
            ["BuildTools", "Community", "Professional", "Enterprise"]
                .map(|edition| base.join("Microsoft Visual Studio/2022").join(edition))
        })
        .collect();
    let setup = first_file(
        vs_roots
            .iter()
            .map(|root| root.join("Common7/Tools/VsDevCmd.bat")),
    );
    let specs = [
        (
            "rustc",
            "rustc.exe",
            vec![user.join(".cargo/bin/rustc.exe")],
        ),
        (
            "cargo",
            "cargo.exe",
            vec![user.join(".cargo/bin/cargo.exe")],
        ),
        (
            "python312",
            "python.exe",
            vec![local.join("Programs/Python/Python312/python.exe")],
        ),
        ("node", "node.exe", vec![program.join("nodejs/node.exe")]),
        (
            "ruby",
            "ruby.exe",
            vec![PathBuf::from("C:/Ruby33-x64/bin/ruby.exe")],
        ),
        (
            "docker",
            "docker.exe",
            vec![program.join("Docker/Docker/resources/bin/docker.exe")],
        ),
        (
            "vswhere",
            "vswhere.exe",
            vec![program_x86.join("Microsoft Visual Studio/Installer/vswhere.exe")],
        ),
        (
            "cmake",
            "cmake.exe",
            vs_roots
                .iter()
                .map(|root| {
                    root.join("Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe")
                })
                .collect(),
        ),
        (
            "ninja",
            "ninja.exe",
            vs_roots
                .iter()
                .map(|root| {
                    root.join("Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja/ninja.exe")
                })
                .collect(),
        ),
    ];
    let mut tools: Vec<_> = specs
        .into_iter()
        .map(|(name, exe, extra)| Tool {
            name,
            path: if name == "python312" {
                first_file(extra)
            } else {
                candidates(exe, extra)
            },
            runtime_validated: false,
            version: None,
        })
        .collect();
    if probe_versions && cfg!(windows) {
        for tool in &mut tools {
            if tool.name == "vswhere" {
                continue;
            }
            tool.version = Some(match &tool.path {
                Some(path) => probe::run(
                    path,
                    tool.name,
                    Duration::from_secs(30)
                        .saturating_sub(start.elapsed())
                        .min(Duration::from_secs(3)),
                ),
                None => probe::Version::absent("missing"),
            });
        }
    }
    let python = tools
        .iter()
        .find(|tool| tool.name == "python312")
        .and_then(|tool| tool.path.as_ref());
    let python_sdk = python
        .map(|path| python_sdk_files(path.as_path()))
        .unwrap_or_else(|| python_sdk_files(std::path::Path::new("")));
    let python_ready = tools
        .iter()
        .find(|tool| tool.name == "python312")
        .and_then(|tool| tool.version.as_ref())
        .is_some_and(|version| version.expected_profile)
        && python_sdk.iter().all(|file| file.path.is_some());
    let child_environment = python
        .filter(|_| python_ready)
        .map(|path| vec![("PYO3_PYTHON", path.to_string_lossy().into_owned())])
        .unwrap_or_default();
    Report {
        schema: "humanymous.windows-doctor.v1",
        windows_host: cfg!(windows),
        tools,
        child_environment,
        msvc_setup: setup,
        msvc_setup_arguments: ["-arch=x64", "-host_arch=x64"],
        linux_container_arguments: ["--context", "desktop-linux"],
        python_sdk,
        ruby_extension_abi_qualified: false,
        complete: false,
        notes: vec![
            "Default discovery is presence-only; --probe executes fixed bounded version commands, not SDK linking or imports.",
            "Invoke VsDevCmd.bat for x64 in a child cmd environment before MSVC builds; do not mutate global environment.",
            "Use explicit discovered executables; inherited PATH may predate user PATH updates.",
            "Ruby mingw-ucrt does not establish compatibility with an MSVC Rust extension; retain a missing ABI profile.",
            "Use Windows-native unit/process tests and explicit Windows docker.exe --context desktop-linux for Linux container gates.",
            "Retain genuine CI OS gates. Do not invoke WSL or switch Docker backend.",
        ],
    }
}

fn python_sdk_files(executable: &std::path::Path) -> Vec<SdkFile> {
    [
        ("Python.h", "include/Python.h"),
        ("python312.lib", "libs/python312.lib"),
    ]
    .into_iter()
    .map(|(name, relative)| SdkFile {
        name,
        path: executable
            .parent()
            .filter(|root| root.is_absolute())
            .and_then(|root| first_file([root.join(relative)])),
        link_validated: false,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_python_sdk_never_qualifies_linking() {
        let files = python_sdk_files(std::path::Path::new("relative/python.exe"));
        assert!(
            files
                .iter()
                .all(|file| file.path.is_none() && !file.link_validated)
        );
        assert_eq!(files.len(), 2);
    }
}
