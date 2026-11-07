//! Hosted-only C/C++ compiler layout conformance for an upstream-generated header.
//! No authored foreign code, unsafe loader or actual dynamic ABI call is present.
#![forbid(unsafe_code)]

use humanymous_native_abi::{Command as NativeCommand, Reply, Response, Version};
use std::{
    collections::BTreeMap,
    io::Write,
    path::Path,
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
    // Root CI owns source/header/tool hashes and timeout. These are trusted
    // compiler invocations on an owned generated header, not arbitrary probes.
    for (tool, language, standard, label) in [
        ("clang", "c", "c11", "c"),
        ("clang++", "c++", "c++11", "cpp"),
    ] {
        for (argument, suffix) in [("--version", "version"), ("-dumpmachine", "target")] {
            let result = Command::new(tool)
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
        let result = Command::new(tool)
            .args([
                "-x",
                language,
                "-fsyntax-only",
                "-Xclang",
                "-fdump-record-layouts-complete",
            ])
            .arg(format!("-std={standard}"))
            .arg(&header)
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
    }
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
