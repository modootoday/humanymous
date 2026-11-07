//! Generate Rust declarations from the pinned host headers; no authored C glue.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct Includes(Arc<Mutex<std::collections::BTreeSet<PathBuf>>>);
impl bindgen::callbacks::ParseCallbacks for Includes {
    fn include_file(&self, filename: &str) {
        let path = PathBuf::from(filename);
        println!("cargo:rerun-if-changed={}", path.display());
        self.0.lock().expect("header inventory lock").insert(path);
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_HTTPD");
    if std::env::var_os("CARGO_FEATURE_HTTPD").is_none() {
        return;
    }
    generate();
}

fn generate() {
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;
    let target = std::env::var("TARGET").expect("Cargo target");
    assert_eq!(
        target, "x86_64-unknown-linux-gnu",
        "unqualified Apache target"
    );
    for name in [
        "HUMANYMOUS_HTTPD_INCLUDE",
        "HUMANYMOUS_APR_INCLUDE",
        "HUMANYMOUS_APU_INCLUDE",
        "HUMANYMOUS_CLANG_LIBRARY",
        "LIBCLANG_PATH",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../adapters/apache");
    let lock_path = root.join("upstream-lock.json");
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let lock: serde_json::Value =
        serde_json::from_slice(&std::fs::read(lock_path).expect("upstream lock"))
            .expect("upstream JSON");
    let directories: Vec<PathBuf> = [
        "HUMANYMOUS_HTTPD_INCLUDE",
        "HUMANYMOUS_APR_INCLUDE",
        "HUMANYMOUS_APU_INCLUDE",
    ]
    .into_iter()
    .map(|name| PathBuf::from(std::env::var_os(name).expect("explicit host include root")))
    .collect();
    assert!(
        directories.iter().all(|path| path.is_absolute()),
        "absolute host include roots required"
    );
    let hash = |path: &Path| {
        format!(
            "{:x}",
            Sha256::digest(std::fs::read(path).expect("read ABI input"))
        )
    };
    let mut headers = BTreeMap::new();
    for (name, expected) in lock["headers"].as_object().expect("header inventory") {
        let path = directories
            .iter()
            .map(|directory| directory.join(name))
            .find(|path| path.is_file())
            .expect("pinned header missing");
        let observed = hash(&path);
        assert_eq!(
            Some(observed.as_str()),
            expected.as_str(),
            "host header differs from pinned release: {name}"
        );
        println!("cargo:rerun-if-changed={}", path.display());
        headers.insert(path.display().to_string(), observed);
    }
    let clang = PathBuf::from(
        std::env::var_os("HUMANYMOUS_CLANG_LIBRARY").expect("explicit libclang library"),
    );
    assert!(
        clang.is_absolute() && clang.is_file(),
        "exact libclang library required"
    );
    assert_eq!(
        std::fs::canonicalize(&clang).expect("configured clang path"),
        std::fs::canonicalize(
            std::env::var_os("LIBCLANG_PATH")
                .expect("LIBCLANG_PATH must identify the same exact library")
        )
        .expect("libclang loader path"),
        "generator must load the recorded library"
    );
    let includes = Arc::new(Mutex::new(std::collections::BTreeSet::new()));
    let mut builder = bindgen::Builder::default()
        .default_macro_constant_type(bindgen::MacroTypeVariation::Signed)
        .generate_comments(false)
        .layout_tests(false)
        .derive_debug(false)
        .allowlist_type("(module_struct|module|command_rec|cmd_parms|request_rec|server_rec|apr_.*|ap_filter.*|ap_input_mode_t|cmd_func).*")
        .allowlist_function("(ap_hook_.*|ap_.*filter.*|ap_mpm_query|ap_show_mpm|ap_get_conn_socket|ap_get_client_block|ap_setup_client_block|ap_should_client_block|ap_find_linked_module|apr_.*)")
        .allowlist_var("(AP_.*|APR_.*|MODULE_MAGIC_.*|HTTP_.*|DECLINED|OK|REQUEST_.*|RSRC_CONF)")
        .parse_callbacks(Box::new(Includes(includes.clone())));
    for directory in &directories {
        builder = builder.clang_arg(format!("-I{}", directory.display()));
    }
    for name in [
        "httpd.h",
        "http_config.h",
        "http_protocol.h",
        "http_core.h",
        "util_filter.h",
        "ap_mpm.h",
    ] {
        builder = builder.header(directories[0].join(name).display().to_string());
    }
    let bindings = builder.generate().expect("generate pinned Apache ABI");
    let loaded =
        std::fs::read_to_string("/proc/self/maps").expect("observe actual generator libraries");
    let expected_clang = std::fs::canonicalize(&clang).expect("canonical libclang");
    assert!(
        loaded
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .filter(|name| name.contains("libclang"))
            .any(|name| std::fs::canonicalize(name).is_ok_and(|path| path == expected_clang)),
        "recorded libclang was not observed loaded"
    );
    let closure: BTreeMap<_, _> = includes
        .lock()
        .expect("header closure lock")
        .iter()
        .map(|path| (path.display().to_string(), hash(path)))
        .collect();
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    bindings
        .write_to_file(output.join("apache_bindings.rs"))
        .expect("write generated Rust ABI");
    let receipt = serde_json::json!({"schema":"humanymous.apache.abi-build/0.1", "target":target,
        "bindgenVersion":"0.73.2", "libclangSha256":hash(&clang),"clangVersion":bindgen::clang_version().full,
        "directHeaders":headers,"configuredHeaderClosure":closure,
        "headerClosureSha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&closure).expect("header closure JSON"))),
        "bindingsSha256":hash(&output.join("apache_bindings.rs")),"fullProductSuccess":false});
    std::fs::write(
        output.join("apache-abi-build.json"),
        serde_json::to_vec_pretty(&receipt).expect("ABI receipt JSON"),
    )
    .expect("write ABI receipt");
}
