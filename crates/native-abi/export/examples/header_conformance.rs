//! Hosted-only C/C++ compiler layout conformance for an upstream-generated header.
//! No authored foreign code, unsafe loader or actual dynamic ABI call is present.
#![forbid(unsafe_code)]

use humanymous_native_abi::{Command as NativeCommand, Reply, Response, Version};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

struct Shape {
    name: &'static str,
    size: usize,
    align: usize,
    fields: &'static [(&'static str, usize)],
}
const SHAPES: [Shape; 4] = [
    Shape {
        name: "HumanymousNativeVersionV1",
        size: std::mem::size_of::<Version>(),
        align: std::mem::align_of::<Version>(),
        fields: &[
            ("major", std::mem::offset_of!(Version, major)),
            ("minor", std::mem::offset_of!(Version, minor)),
            ("struct_size", std::mem::offset_of!(Version, struct_size)),
            ("reserved", std::mem::offset_of!(Version, reserved)),
        ],
    },
    Shape {
        name: "HumanymousNativeReplyV1",
        size: std::mem::size_of::<Reply>(),
        align: std::mem::align_of::<Reply>(),
        fields: &[
            ("status", std::mem::offset_of!(Reply, status)),
            ("reserved", std::mem::offset_of!(Reply, reserved)),
            ("value", std::mem::offset_of!(Reply, value)),
        ],
    },
    Shape {
        name: "HumanymousNativeCommandV1",
        size: std::mem::size_of::<NativeCommand>(),
        align: std::mem::align_of::<NativeCommand>(),
        fields: &[
            ("version", std::mem::offset_of!(NativeCommand, version)),
            ("operation", std::mem::offset_of!(NativeCommand, operation)),
            ("flags", std::mem::offset_of!(NativeCommand, flags)),
            ("context", std::mem::offset_of!(NativeCommand, context)),
            ("buffer", std::mem::offset_of!(NativeCommand, buffer)),
            ("offset", std::mem::offset_of!(NativeCommand, offset)),
            ("length", std::mem::offset_of!(NativeCommand, length)),
            ("data", std::mem::offset_of!(NativeCommand, data)),
        ],
    },
    Shape {
        name: "HumanymousNativeResponseV1",
        size: std::mem::size_of::<Response>(),
        align: std::mem::align_of::<Response>(),
        fields: &[
            ("reply", std::mem::offset_of!(Response, reply)),
            ("data", std::mem::offset_of!(Response, data)),
        ],
    },
];

#[derive(Default)]
struct Layout {
    size: usize,
    align: usize,
    fields: BTreeMap<String, usize>,
}

fn number(text: &str, key: &str) -> Result<usize, &'static str> {
    let tail = text.split_once(key).ok_or("missing layout metadata")?.1;
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().map_err(|_| "invalid layout metadata")
}

fn layouts(text: &str) -> Result<BTreeMap<String, Layout>, &'static str> {
    let mut result = BTreeMap::new();
    let mut current: Option<(String, Layout)> = None;
    for line in text.lines() {
        let Some((offset, right)) = line.split_once('|') else {
            continue;
        };
        let content = right.trim_start();
        let indent = right.len() - content.len();
        if indent == 1 && content.starts_with("struct ") {
            if current.is_some() {
                return Err("unterminated compiler layout");
            }
            let name = content.trim_start_matches("struct ");
            if SHAPES.iter().any(|shape| shape.name == name) {
                current = Some((name.to_owned(), Layout::default()));
            }
            continue;
        }
        let Some((_, layout)) = current.as_mut() else {
            continue;
        };
        if content.starts_with("[sizeof=") {
            layout.size = number(content, "sizeof=")?;
            layout.align = number(content, "align=")?;
            let (name, layout) = current.take().ok_or("missing compiler layout")?;
            if result.insert(name, layout).is_some() {
                return Err("duplicate compiler layout");
            }
        } else if indent == 3 {
            let field = content
                .split_whitespace()
                .last()
                .ok_or("missing compiler field")?;
            let offset = offset
                .trim()
                .parse()
                .map_err(|_| "invalid compiler field offset")?;
            if layout.fields.insert(field.to_owned(), offset).is_some() {
                return Err("duplicate compiler field");
            }
        }
    }
    if current.is_some() {
        return Err("unterminated compiler layout");
    }
    Ok(result)
}

fn verify(text: &str) -> Result<(), &'static str> {
    let actual = layouts(text)?;
    for expected in SHAPES {
        let found = actual
            .get(expected.name)
            .ok_or("required compiler layout absent")?;
        let fields: BTreeMap<_, _> = expected
            .fields
            .iter()
            .map(|&(name, offset)| (name.to_owned(), offset))
            .collect();
        if found.size != expected.size || found.align != expected.align || found.fields != fields {
            return Err("compiler layout differs from Rust layout");
        }
    }
    Ok(())
}

fn save(directory: &Path, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(name))
        .map_err(|_| "cannot create new compiler evidence file")?;
    file.write_all(bytes)
        .map_err(|_| "cannot write compiler evidence")
}

fn public_tool_path(path: &Path) -> Result<&str, &'static str> {
    let text = path.to_str().ok_or("compiler path is not UTF-8")?;
    if text.len() > 1024
        || !text.starts_with('/')
        || text.split('/').any(|part| part == "." || part == "..")
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/_.+-".contains(&byte))
    {
        return Err("compiler path is outside the bounded metadata profile");
    }
    Ok(text)
}

fn parse_digest(bytes: &[u8], path: &Path) -> Result<String, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "compiler digest is not UTF-8")?;
    let (digest, suffix) = text.split_at_checked(64).ok_or("compiler digest absent")?;
    if !digest
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || suffix != format!("  {}\n", public_tool_path(path)?)
    {
        return Err("compiler digest response refused");
    }
    Ok(digest.to_owned())
}

fn binary_digest(path: &Path) -> Result<String, &'static str> {
    public_tool_path(path)?;
    let result = Command::new("/usr/bin/sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .map_err(|_| "stock compiler digest helper unavailable")?;
    if !result.status.success() || !result.stderr.is_empty() || result.stdout.len() > 2048 {
        return Err("stock compiler digest helper failed");
    }
    parse_digest(&result.stdout, path)
}

struct Compiler {
    invocation: PathBuf,
    resolved: PathBuf,
    before: String,
}
impl Compiler {
    fn resolve(name: &str) -> Result<Self, &'static str> {
        let search = std::env::var_os("PATH").ok_or("compiler PATH unavailable")?;
        for directory in std::env::split_paths(&search) {
            if !directory.is_absolute() {
                return Err("relative compiler search directory refused");
            }
            let invocation = directory.join(name);
            if !invocation.is_file() {
                continue;
            }
            public_tool_path(&invocation)?;
            let resolved = invocation
                .canonicalize()
                .map_err(|_| "compiler canonical path unavailable")?;
            let before = binary_digest(&resolved)?;
            return Ok(Self {
                invocation,
                resolved,
                before,
            });
        }
        Err("compiler executable unavailable")
    }

    fn identity(&self, label: &str) -> Result<String, &'static str> {
        let current = self
            .invocation
            .canonicalize()
            .map_err(|_| "compiler path disappeared")?;
        if current != self.resolved {
            return Err("compiler resolved path changed");
        }
        let after = binary_digest(&current)?;
        if after != self.before {
            return Err("compiler binary digest changed");
        }
        Ok(format!(
            "{label}.path={}\n{label}.sha256.before={}\n{label}.sha256.after={after}\n",
            public_tool_path(&self.resolved)?,
            self.before,
        ))
    }
}

fn argv_record(command: &Command) -> Result<String, &'static str> {
    let program = public_tool_path(Path::new(command.get_program()))?;
    let args: Vec<_> = command.get_args().collect();
    if args.len() != 7 {
        return Err("compiler argv denominator refused");
    }
    let mut text = format!("schema=humanymous.header-compiler-argv/1\nprogram={program}\n");
    for (index, arg) in args.iter().enumerate() {
        let arg = arg.to_str().ok_or("compiler argv is not UTF-8")?;
        if arg.len() > 1024
            || arg.is_empty()
            || !arg
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"/_.+-=".contains(&byte))
        {
            return Err("compiler argv outside bounded metadata profile");
        }
        text.push_str(&format!("arg{index:02}={arg}\n"));
    }
    public_tool_path(Path::new(args[6]))?;
    Ok(text)
}

fn save_argv(output: &Path, label: &str, command: &Command) -> Result<(), &'static str> {
    let text = argv_record(command)?;
    let path = output.with_extension(format!("{label}-argv.txt"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "fresh private compiler argv sibling required")?;
    file.write_all(text.as_bytes())
        .map_err(|_| "cannot retain private compiler argv")
}

fn run() -> Result<(), &'static str> {
    // Environment checks prevent accidental local execution; they are not an
    // attestation against a malicious host that can forge its own environment.
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
        || std::env::var("RUNNER_ENVIRONMENT").as_deref() != Ok("github-hosted")
        || std::env::consts::OS != "linux"
        || std::env::consts::ARCH != "x86_64"
    {
        return Err("requires standard GitHub-hosted Linux x86_64; not_run");
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("expected generated-header and fresh-evidence-directory paths");
    }
    let header = std::fs::canonicalize(&args[0]).map_err(|_| "generated header unavailable")?;
    let metadata =
        std::fs::metadata(&header).map_err(|_| "generated header metadata unavailable")?;
    if !metadata.is_file() || metadata.len() > 131_072 {
        return Err("generated header exceeds bounded file contract");
    }
    let declarations = std::fs::read_to_string(&header)
        .map_err(|_| "generated header is not bounded UTF-8 text")?;
    if !declarations.contains("humanymous_native_abi_v1(") {
        return Err("versioned entrypoint declaration absent from generated header");
    }
    let output = Path::new(&args[1]);
    std::fs::create_dir(output).map_err(|_| "fresh evidence directory required")?;
    let mut identities = String::from("schema=humanymous.header-compiler-identity/1\n");
    // Root CI owns source/header/tool hashes and timeout. These are trusted
    // compiler invocations on an owned generated header, not arbitrary probes.
    for (tool, language, standard, label) in [
        ("clang", "c", "c11", "c"),
        ("clang++", "c++", "c++11", "cpp"),
    ] {
        // Preserve the invocation basename for the clang++ driver while hashing
        // the resolved executable. Trusted immutable tool storage is a premise;
        // sampled hashes are not atomic process-image or hostile-host attestation.
        let compiler = Compiler::resolve(tool)?;
        for (argument, suffix) in [("--version", "version"), ("-dumpmachine", "target")] {
            let result = Command::new(&compiler.invocation)
                .arg(argument)
                .output()
                .map_err(|_| "compiler metadata unavailable")?;
            save(output, &format!("{label}-{suffix}.txt"), &result.stdout)?;
            save(
                output,
                &format!("{label}-{suffix}-stderr.txt"),
                &result.stderr,
            )?;
            if !result.status.success() {
                return Err("compiler metadata failed");
            }
        }
        let mut command = Command::new(&compiler.invocation);
        command
            .args([
                "-x",
                language,
                "-fsyntax-only",
                "-Xclang",
                "-fdump-record-layouts-complete",
            ])
            .arg(format!("-std={standard}"))
            .arg(&header);
        // Observe this exact Command, including its private actual header path;
        // only its digest and labelled template may enter the public context.
        save_argv(output, label, &command)?;
        let result = command
            .output()
            .map_err(|_| "compiler execution unavailable")?;
        save(output, &format!("{label}-stdout.txt"), &result.stdout)?;
        save(output, &format!("{label}-stderr.txt"), &result.stderr)?;
        if !result.status.success() {
            return Err("generated header compilation failed");
        }
        let mut text =
            String::from_utf8(result.stdout).map_err(|_| "compiler output is not UTF-8")?;
        text.push_str(
            &String::from_utf8(result.stderr).map_err(|_| "compiler diagnostics are not UTF-8")?,
        );
        verify(&text)?;
        identities.push_str(&compiler.identity(label)?);
    }
    // Keep the closed thirteen-file compiler directory unchanged. The separate
    // sibling is typed metadata for Delivery's context, not a seventeenth payload.
    let identity_path = output.with_extension("identity.txt");
    let mut identity = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(identity_path)
        .map_err(|_| "fresh sibling compiler identity file required")?;
    identity
        .write_all(identities.as_bytes())
        .map_err(|_| "cannot write compiler identity metadata")?;
    save(output,"scope.txt",b"C11 and C++11 header syntax and record layouts match Rust. Actual dynamic loading, RAII, exceptions, allocator host glue and product e2e: not_run.\n")?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => {
            println!(
                "C/C++ generated header layout conformance passed; actual dynamic ABI not_run"
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_command_argv_is_observed_without_template_substitution() {
        let mut command = Command::new("/usr/bin/clang++");
        command.args([
            "-x",
            "c++",
            "-fsyntax-only",
            "-Xclang",
            "-fdump-record-layouts-complete",
            "-std=c++11",
            "/private-run/native-abi-v1.h",
        ]);
        let text = argv_record(&command).unwrap();
        assert_eq!(text.lines().count(), 9);
        assert!(
            text.starts_with(
                "schema=humanymous.header-compiler-argv/1\nprogram=/usr/bin/clang++\n"
            )
        );
        assert!(text.ends_with("arg06=/private-run/native-abi-v1.h\n"));
        command.arg("extra");
        assert!(argv_record(&command).is_err());
    }

    #[test]
    fn private_command_argv_refuses_control_injection_and_relative_header() {
        for header in ["relative.h", "/private/a b.h", "/private/a\narg07=extra"] {
            let mut command = Command::new("/usr/bin/clang");
            command.args([
                "-x",
                "c",
                "-fsyntax-only",
                "-Xclang",
                "-fdump-record-layouts-complete",
                "-std=c11",
                header,
            ]);
            assert!(argv_record(&command).is_err());
        }
    }

    #[test]
    fn compiler_identity_digest_requires_exact_path_lowercase_hex_and_one_line() {
        let path = Path::new("/usr/lib/llvm-18/bin/clang");
        let digest = "a".repeat(64);
        let good = format!("{digest}  /usr/lib/llvm-18/bin/clang\n");
        assert_eq!(parse_digest(good.as_bytes(), path).unwrap(), digest);
        for invalid in [
            good.trim_end_matches('\n').to_owned(),
            good.replace('a', "A"),
            good.replace("/usr/lib/llvm-18/bin/clang", "/usr/bin/other"),
            format!("{good}{good}"),
            format!("\\{good}"),
            good.replace("  /", " /"),
        ] {
            assert!(parse_digest(invalid.as_bytes(), path).is_err());
        }
        assert!(parse_digest(&[255; 100], path).is_err());
    }

    #[test]
    fn compiler_metadata_paths_refuse_controls_relative_and_noncanonical_components() {
        assert!(public_tool_path(Path::new("/usr/lib/llvm-18/bin/clang")).is_ok());
        for path in [
            "clang",
            "/usr/bin/a b",
            "/usr/bin/a\n",
            "/usr/bin/a\t",
            "/usr/../bin/clang",
            "/usr/./bin/clang",
            "/usr/bin/\u{1b}clang",
            "/usr/bin/é",
        ] {
            assert!(public_tool_path(Path::new(path)).is_err());
        }
    }

    // Synthetic compiler diagnostics only; this is not executable C/C++ source.
    fn fixture(cpp: bool) -> String {
        let mut output = String::new();
        for shape in SHAPES {
            output.push_str(&format!("         0 | struct {}\n", shape.name));
            for &(field, offset) in shape.fields {
                output.push_str(&format!("{offset:10} |   diagnostic_type {field}\n"));
            }
            if cpp {
                output.push_str(&format!("           | [sizeof={}, dsize={}, align={},\n           |  nvsize={}, nvalign={}]\n",shape.size,shape.size,shape.align,shape.size,shape.align));
            } else {
                output.push_str(&format!(
                    "           | [sizeof={}, align={}]\n",
                    shape.size, shape.align
                ));
            }
        }
        output
    }
    #[test]
    fn c_and_cpp_diagnostic_forms_match_rust_layouts() {
        verify(&fixture(false)).unwrap();
        verify(&fixture(true)).unwrap();
    }
    #[test]
    fn missing_duplicate_and_malformed_layouts_fail_closed() {
        let text = fixture(false);
        assert!(verify("").is_err());
        assert!(verify(&(text.clone() + &text)).is_err());
        assert!(verify(&text.replacen("sizeof=16", "sizeof=invalid", 1)).is_err());
        assert!(
            verify(&text.replacen("struct HumanymousNativeVersionV1", "struct Unknown", 1))
                .is_err()
        );
        assert!(
            verify(&text.replacen("diagnostic_type major", "diagnostic_type other", 1)).is_err()
        );
    }
    #[test]
    fn size_alignment_and_field_offset_changes_reject() {
        let text = fixture(false);
        for changed in [
            text.replacen("sizeof=16", "sizeof=17", 1),
            text.replacen("align=4", "align=8", 1),
            text.replacen(
                "         0 |   diagnostic_type major",
                "         1 |   diagnostic_type major",
                1,
            ),
        ] {
            assert!(verify(&changed).is_err());
        }
    }
}
