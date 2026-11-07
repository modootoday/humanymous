//! Same-source Linux container measurements; no publication or hosted identity spoof.
#![forbid(unsafe_code)]
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, String>;
fn quoted(value: &str) -> String {
    let mut result = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c < ' ' => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
fn object(fields: &[(&str, String)]) -> String {
    format!(
        "{{{}}}",
        fields
            .iter()
            .map(|(key, value)| format!("{}:{value}", quoted(key)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn optional(value: Option<String>) -> String {
    value.map(|v| quoted(&v)).unwrap_or_else(|| "null".into())
}
fn number(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".into())
}
fn text(root: &Path, program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| format!("cannot start {program}"))?;
    if !output.status.success() {
        return Err(format!("{program} metadata command failed"));
    }
    String::from_utf8(output.stdout)
        .map(|v| v.trim().to_owned())
        .map_err(|_| "metadata is not UTF-8".into())
}
fn sha(path: &Path) -> Option<String> {
    let output = Command::new("sha256sum")
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let value = String::from_utf8(output.stdout)
        .ok()?
        .split_whitespace()
        .next()?
        .to_owned();
    (output.status.success()
        && value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)))
    .then_some(value)
}
fn exact_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn clean(root: &Path, scopes: &[&str]) -> bool {
    let mut args = vec!["status", "--porcelain=v1", "--untracked-files=all", "--"];
    args.extend_from_slice(scopes);
    text(root, "git", &args).is_ok_and(|value| value.is_empty())
}
fn context(root: &Path) -> Result<String> {
    let local = std::env::var_os("ACT").is_some();
    let local_provider = std::env::var("HUMANYMOUS_BENCHMARK_LOCAL_PROVIDER").ok();
    let local_host = std::env::var("HUMANYMOUS_BENCHMARK_DOCKER_HOST").ok();
    let local_engine = std::env::var("HUMANYMOUS_BENCHMARK_LOCAL_ENGINE_ID").ok();
    if local
        && (local_provider.as_deref() != Some("windows-act-desktop-linux")
            || local_host.as_deref() != Some("npipe:////./pipe/dockerDesktopLinuxEngine")
            || !local_engine.as_ref().is_some_and(|id| {
                !id.is_empty()
                    && id.len() <= 256
                    && id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'-'))
            }))
    {
        return Err("local ACT requires the verified Windows Docker Desktop Linux-engine preflight; WSL Docker fallback is forbidden".into());
    }
    let memory = fs::read_to_string("/sys/fs/cgroup/memory.max")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok());
    let swap = fs::read_to_string("/sys/fs/cgroup/memory.swap.max")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok());
    let cpu = fs::read_to_string("/sys/fs/cgroup/cpu.max").unwrap_or_default();
    let values: Vec<_> = cpu.split_whitespace().collect();
    let quota = values.first().and_then(|v| v.parse::<u64>().ok());
    let period = values.get(1).and_then(|v| v.parse::<u64>().ok());
    if !memory.is_some_and(|v| v > 0 && v <= 2_147_483_648)
        || swap != Some(0)
        || !quota
            .zip(period)
            .is_some_and(|(q, p)| q > 0 && p > 0 && q <= p.saturating_mul(4))
    {
        return Err(
            "actual cgroup limits must enforce at most 4 CPUs, 2 GiB memory and no additional swap"
                .into(),
        );
    }
    let nproc = text(root, "nproc", &[])?;
    let uid = text(root, "id", &["-u"])?;
    let mut fields = vec![
        (
            "kind",
            quoted(if std::env::var_os("ACT").is_some() {
                "local-act-container"
            } else if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
                && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            {
                "github-hosted-container"
            } else {
                "unverified-linux-context"
            }),
        ),
        ("os", quoted(std::env::consts::OS)),
        ("arch", quoted(std::env::consts::ARCH)),
        (
            "image_ref_requested",
            optional(std::env::var("HUMANYMOUS_BENCHMARK_IMAGE").ok()),
        ),
        ("image_identity_verified_in_process", "false".into()),
        ("vm_equivalence_claimed", "false".into()),
        ("local_provider", optional(local_provider)),
        ("local_docker_host", optional(local_host)),
        (
            "local_engine_id_reported_by_outer_preflight",
            optional(local_engine),
        ),
        ("local_engine_attested_in_process", "false".into()),
        ("nproc", quoted(&nproc)),
        ("uid", quoted(&uid)),
        ("rustc", optional(text(root, "rustc", &["-vV"]).ok())),
        ("cargo", optional(text(root, "cargo", &["-V"]).ok())),
        (
            "runner_image_version",
            optional(std::env::var("ImageVersion").ok()),
        ),
    ];
    for (key, path) in [
        ("cpu_max", "/sys/fs/cgroup/cpu.max"),
        ("memory_max", "/sys/fs/cgroup/memory.max"),
        ("memory_swap_max", "/sys/fs/cgroup/memory.swap.max"),
        ("memory_peak_whole_cgroup", "/sys/fs/cgroup/memory.peak"),
        ("cpu_stat_whole_cgroup", "/sys/fs/cgroup/cpu.stat"),
    ] {
        fields.push((
            key,
            optional(fs::read_to_string(path).ok().map(|v| v.trim().to_owned())),
        ));
    }
    Ok(object(&fields))
}
fn counts(log: &str) -> Option<[u64; 5]> {
    let lines: Vec<_> = log
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    if lines.len() != 1 {
        return None;
    }
    let mut result = [0; 5];
    for (index, label) in ["passed", "failed", "ignored", "measured", "filtered out"]
        .iter()
        .enumerate()
    {
        result[index] = lines[0].split(';').find_map(|part| {
            part.trim()
                .strip_suffix(label)
                .and_then(|v| v.split_whitespace().last()?.parse().ok())
        })?;
    }
    Some(result)
}
fn resource(path: &Path) -> String {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let mut fields = Vec::new();
    for (key, label) in [
        ("user_seconds", "User time (seconds)"),
        ("system_seconds", "System time (seconds)"),
        (
            "max_rss_kbytes_reported",
            "Maximum resident set size (kbytes)",
        ),
        ("voluntary_context_switches", "Voluntary context switches"),
        (
            "involuntary_context_switches",
            "Involuntary context switches",
        ),
        ("filesystem_inputs", "File system inputs"),
        ("filesystem_outputs", "File system outputs"),
    ] {
        let value = raw
            .lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{label}: ")))
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        fields.push((key, value));
    }
    fields.push((
        "provider",
        quoted("GNU time -v; max RSS is not simultaneous whole-machine memory"),
    ));
    object(&fields)
}
fn resource_complete(path: &Path) -> bool {
    let raw = fs::read_to_string(path).unwrap_or_default();
    [
        "User time (seconds)",
        "System time (seconds)",
        "Maximum resident set size (kbytes)",
    ]
    .iter()
    .all(|label| {
        raw.lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{label}: ")))
            .and_then(|value| value.parse::<f64>().ok())
            .is_some_and(|value| value.is_finite() && value >= 0.0)
    })
}
fn materialize(control: &Path, source: &Path, expected: &str, log: &Path) -> Result<()> {
    let output = fs::File::create(log).map_err(|_| "cannot create materialization log")?;
    let source_text = source.to_str().ok_or("workload path is not UTF-8")?;
    for args in [
        vec!["init", "--quiet", source_text],
        vec![
            "-C",
            source_text,
            "remote",
            "add",
            "origin",
            "https://github.com/modootoday/humanymous.git",
        ],
        vec![
            "-C",
            source_text,
            "-c",
            "credential.helper=",
            "-c",
            "http.extraHeader=",
            "fetch",
            "--depth",
            "1",
            "origin",
            expected,
        ],
        vec![
            "-C",
            source_text,
            "checkout",
            "--quiet",
            "--detach",
            "FETCH_HEAD",
        ],
    ] {
        let status = Command::new("git")
            .args(args)
            .current_dir(control)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdin(Stdio::null())
            .stdout(output.try_clone().map_err(|_| "log clone failed")?)
            .stderr(output.try_clone().map_err(|_| "log clone failed")?)
            .status()
            .map_err(|_| "workload Git command failed to start")?;
        if !status.success() {
            return Err("cannot materialize exact public workload commit".into());
        }
    }
    if text(source, "git", &["rev-parse", "HEAD"])? != expected || !clean(source, &[]) {
        return Err("workload source identity is not clean and exact".into());
    }
    Ok(())
}
fn prepare(control: &Path, directory: &Path) -> Result<String> {
    let mut records = Vec::new();
    for (name, args) in [
        ("time-package-index", vec!["update"]),
        (
            "time-package-install",
            vec!["install", "-y", "--no-install-recommends", "time"],
        ),
    ] {
        let log = fs::File::create(directory.join(format!("{name}.log")))
            .map_err(|_| "cannot retain setup log")?;
        let started = Instant::now();
        let status = Command::new("apt-get")
            .args(args)
            .current_dir(control)
            .env("DEBIAN_FRONTEND", "noninteractive")
            .stdin(Stdio::null())
            .stdout(log.try_clone().map_err(|_| "setup log clone failed")?)
            .stderr(log)
            .status()
            .map_err(|_| "cannot start time package setup")?;
        let record = object(&[
            ("stage", quoted(name)),
            ("elapsed_ms", started.elapsed().as_millis().to_string()),
            (
                "exit_code",
                status
                    .code()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".into()),
            ),
        ]);
        println!("{record}");
        records.push(record);
        if !status.success() {
            return Err("same-container GNU time preparation failed".into());
        }
    }
    let package = text(control, "dpkg-query", &["-W", "-f=${Version}", "time"])?;
    if !Path::new("/usr/bin/time").is_file() {
        return Err("installed GNU time executable unavailable".into());
    }
    Ok(object(&[
        (
            "provider",
            quoted("same explicit apt-get update/install time in both venues"),
        ),
        ("package_version", quoted(&package)),
        ("stages", format!("[{}]", records.join(","))),
    ]))
}
fn run() -> Result<bool> {
    if std::env::args_os().len() != 1 {
        return Err("the benchmark driver accepts only its declared environment inputs".into());
    }
    if !cfg!(target_os = "linux") {
        return Err("this workflow comparison requires Linux containers".into());
    }
    let expected = std::env::var("HUMANYMOUS_BENCHMARK_SOURCE")
        .map_err(|_| "exact workload source is required")?;
    let expected_control = std::env::var("HUMANYMOUS_BENCHMARK_CONTROL_SHA")
        .map_err(|_| "exact control source is required")?;
    if !exact_sha(&expected) || !exact_sha(&expected_control) {
        return Err("source identifiers must be exact lowercase commit SHAs".into());
    }
    let passes = std::env::var("HUMANYMOUS_BENCHMARK_PASSES")
        .unwrap_or_else(|_| "2".into())
        .parse::<u8>()
        .map_err(|_| "invalid pass count")?;
    if !(1..=2).contains(&passes) {
        return Err("one or two passes are supported".into());
    }
    let control = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .ok_or("control root missing")?
        .to_owned();
    let scopes = [
        "scripts/ci/local-parity",
        ".github/workflows/local-parity.yml",
        "scripts/ci/workflows.json",
    ];
    let control_sha = text(&control, "git", &["rev-parse", "HEAD"])?;
    if control_sha != expected_control || !clean(&control, &scopes) {
        return Err("control source is not clean and exact".into());
    }
    if !text(&control, "rustc", &["--version"])?.starts_with("rustc 1.99.1 ") {
        return Err("actual benchmark toolchain must be Rust 1.99.1".into());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock before epoch")?
        .as_nanos();
    let directory = control
        .join("target/local-parity-evidence")
        .join(format!("run-{nonce}-{}", std::process::id()));
    fs::create_dir_all(directory.parent().ok_or("evidence parent missing")?)
        .map_err(|_| "cannot create evidence parent")?;
    fs::create_dir(&directory).map_err(|_| "fresh evidence directory required")?;
    let environment = context(&control)?;
    println!(
        "{}",
        object(&[
            ("schema", quoted("humanymous.local-parity-start/1")),
            ("control_source_sha", quoted(&control_sha)),
            ("workload_source_sha_requested", quoted(&expected)),
            ("context", environment.clone())
        ])
    );
    let preparation = prepare(&control, &directory)?;
    let source = directory.join("workload");
    let start = Instant::now();
    materialize(
        &control,
        &source,
        &expected,
        &directory.join("materialization.log"),
    )?;
    let materialization_ms = start.elapsed().as_millis();
    let lock = sha(&source.join("Cargo.lock")).ok_or("workload lock hash unavailable")?;
    let mut records = Vec::new();
    let mut all_ok = true;
    let mut source_changed = false;
    for pass in 0..passes {
        for (suite, args, expected_tests) in [
            (
                "collector-lib",
                vec![
                    "test",
                    "--locked",
                    "-p",
                    "humanymous-collector",
                    "--lib",
                    "--",
                    "--test-threads=1",
                ],
                Some(26),
            ),
            (
                "workspace-all-targets",
                vec!["build", "--locked", "--workspace", "--all-targets"],
                None,
            ),
            (
                "runtime-native-process",
                vec![
                    "test",
                    "--locked",
                    "-p",
                    "humanymous-runtime",
                    "--test",
                    "native_process",
                    "--",
                    "--test-threads=1",
                ],
                Some(1),
            ),
        ] {
            if source_changed {
                records.push(object(&[
                    ("suite", quoted(suite)),
                    ("pass", pass.to_string()),
                    ("status", quoted("not_run_source_changed")),
                ]));
                continue;
            }
            let prefix = format!("pass-{pass}-{suite}");
            let stdout = directory.join(format!("{prefix}.stdout.log"));
            let stderr = directory.join(format!("{prefix}.stderr.log"));
            let resources = directory.join(format!("{prefix}.time.txt"));
            let out = fs::File::create(&stdout).map_err(|_| "cannot create stdout log")?;
            let err = fs::File::create(&stderr).map_err(|_| "cannot create stderr log")?;
            let target_present = source.join("target").exists();
            println!(
                "{}",
                object(&[
                    ("schema", quoted("humanymous.local-parity-phase/1")),
                    ("phase", quoted("started")),
                    ("suite", quoted(suite)),
                    ("pass", pass.to_string()),
                    ("workload_source_sha", quoted(&expected)),
                    ("target_existed_before", target_present.to_string())
                ])
            );
            let started = Instant::now();
            let result = Command::new("/usr/bin/time")
                .arg("-v")
                .arg("-o")
                .arg(&resources)
                .arg("--")
                .arg("cargo")
                .args(&args)
                .current_dir(&source)
                .env("LC_ALL", "C")
                .env("CARGO_BUILD_JOBS", "1")
                .env("CARGO_INCREMENTAL", "0")
                .env("CARGO_TARGET_DIR", source.join("target"))
                .stdin(Stdio::null())
                .stdout(out)
                .stderr(err)
                .status();
            let elapsed_ms = started.elapsed().as_millis();
            let exit = result.as_ref().ok().and_then(|status| status.code());
            let actual = if expected_tests.is_some() {
                fs::read_to_string(&stdout)
                    .ok()
                    .and_then(|text| counts(&text))
            } else {
                None
            };
            source_changed = !clean(&source, &[])
                || text(&source, "git", &["rev-parse", "HEAD"])? != expected
                || sha(&source.join("Cargo.lock")).as_deref() != Some(lock.as_str())
                || !clean(&control, &scopes);
            let out_hash = sha(&stdout);
            let err_hash = sha(&stderr);
            let time_hash = sha(&resources);
            let metrics_complete = resource_complete(&resources);
            let valid = exit == Some(0)
                && expected_tests.is_none_or(|count| actual == Some([count, 0, 0, 0, 0]))
                && !source_changed
                && out_hash.is_some()
                && err_hash.is_some()
                && time_hash.is_some()
                && metrics_complete;
            all_ok &= valid;
            let record = object(&[
                ("suite", quoted(suite)),
                ("pass", pass.to_string()),
                ("status", quoted(if valid { "passed" } else { "failed" })),
                ("elapsed_ms", elapsed_ms.to_string()),
                (
                    "exit_code",
                    exit.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
                ),
                ("expected_tests", number(expected_tests)),
                (
                    "actual_counts",
                    actual
                        .map(|counts| format!("[{}]", counts.map(|v| v.to_string()).join(",")))
                        .unwrap_or_else(|| "null".into()),
                ),
                (
                    "count_order",
                    quoted("passed,failed,ignored,measured,filtered"),
                ),
                ("target_existed_before", target_present.to_string()),
                ("cache_reuse_qualified", "false".into()),
                ("resource_metrics_complete", metrics_complete.to_string()),
                ("resources", resource(&resources)),
                ("stdout_sha256", optional(out_hash)),
                ("stderr_sha256", optional(err_hash)),
                ("resource_log_sha256", optional(time_hash)),
                ("source_changed", source_changed.to_string()),
            ]);
            println!("{record}");
            records.push(record);
        }
    }
    let mut control_hashes = Vec::new();
    for (label, path) in [
        ("main_rs", "scripts/ci/local-parity/src/main.rs"),
        ("manifest", "scripts/ci/local-parity/Cargo.toml"),
        ("lock", "scripts/ci/local-parity/Cargo.lock"),
        ("workflow", ".github/workflows/local-parity.yml"),
    ] {
        control_hashes.push((label, optional(sha(&control.join(path)))));
    }
    let summary = object(&[
        ("schema", quoted("humanymous.local-parity/1")),
        ("control_source_sha", quoted(&control_sha)),
        ("control_working_byte_hashes", object(&control_hashes)),
        ("workload_source_sha", quoted(&expected)),
        ("workload_lock_sha256", quoted(&lock)),
        ("context", environment),
        ("preparation", preparation),
        ("materialization_ms", materialization_ms.to_string()),
        ("passes_requested", passes.to_string()),
        (
            "suite_order",
            quoted(
                "collector-lib,workspace-all-targets,runtime-native-process; initial then optional same-job repeat",
            ),
        ),
        ("cold_cache_verified", "false".into()),
        ("comparison_performed", "false".into()),
        ("full_product_qualified", "false".into()),
        ("source_changed", source_changed.to_string()),
        ("all_requested_suites_passed", all_ok.to_string()),
        ("records", format!("[{}]", records.join(","))),
        ("raw_logs_uploaded", "false".into()),
    ]);
    fs::write(directory.join("report.json"), format!("{summary}\n"))
        .map_err(|_| "cannot retain benchmark summary")?;
    println!("{summary}");
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|_| "cannot open step summary")?;
        writeln!(
            file,
            "### Same-source workflow measurements\n\n```json\n{summary}\n```"
        )
        .map_err(|_| "cannot write step summary")?;
    }
    Ok(all_ok && !source_changed)
}
fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            println!(
                "{}",
                object(&[
                    ("schema", quoted("humanymous.local-parity/1")),
                    ("status", quoted("preflight_failed")),
                    ("error", quoted(&error)),
                    ("full_product_qualified", "false".into())
                ])
            );
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn count_and_json_boundaries_are_explicit() {
        assert_eq!(
            counts("test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"),
            Some([26, 0, 0, 0, 0])
        );
        assert!(counts("test result: ok. 0 passed;\ntest result: ok. 0 passed;").is_none());
        assert_eq!(quoted("a\n\"\\"), "\"a\\n\\\"\\\\\"");
        assert!(!exact_sha("main"));
        assert!(exact_sha(&"a".repeat(40)));
    }
}
