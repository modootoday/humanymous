//! Ports of legacy cmd/gate/logging_test.go and cmd/server/logging_test.go
//! (baseline 981d25c), plus the internal/logger canonical-record golden and
//! the fail-closed differences documented on the module.
use super::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// 2026-07-26T03:34:56.123456789Z, the Go logger_test fixed time.
const FIXED: Duration = Duration::new(1_785_036_896, 123_456_789);

#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);
impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Shared {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-logging-{tag}-{}", hex::encode(nonce)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn raw<'a>(level: &'a str, format: &'a str, stream: &'a str) -> RawSettings<'a> {
    RawSettings {
        level,
        console_format: format,
        console_stream: stream,
        plain_file: "",
        jsonl_file: "",
    }
}

fn open(role: Role, raw: &RawSettings<'_>) -> Result<(OperationalLogger, Shared, Shared), Refusal> {
    let settings = Settings::validate(role, raw)?;
    let (stdout, stderr) = (Shared::default(), Shared::default());
    let logger = OperationalLogger::open(
        Identity::new(role, "v-test", "node-test"),
        &settings,
        Box::new(stdout.clone()),
        Box::new(stderr.clone()),
    )?;
    Ok((logger, stdout, stderr))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn path_str(path: &Path) -> &str {
    path.to_str().unwrap()
}

// --- cmd/gate/logging_test.go ---

/// TestEnvOrDefault: a present environment value wins over the fallback.
#[test]
fn environment_value_or_fallback() {
    assert_eq!(resolve_setting(None, Some("jsonl"), "plain"), "jsonl");
    assert_eq!(resolve_setting(None, None, "plain"), "plain");
}

/// TestOpenGateLoggerWritesConsolePlainAndJSONLFile.
#[test]
fn gate_writes_console_plain_and_jsonl_file() {
    let dir = TempDir::new("gate");
    let (plain, jsonl) = (dir.file("gate.log"), dir.file("gate.jsonl"));
    let (mut logger, stdout, stderr) = open(
        Role::Gate,
        &RawSettings {
            plain_file: path_str(&plain),
            jsonl_file: path_str(&jsonl),
            ..raw("info", "plain", "stderr")
        },
    )
    .unwrap();
    logger.emit(
        FIXED,
        Level::Info,
        "Gate logging test.",
        &[
            ("component", Value::Str("gate.test")),
            ("event", Value::Str("runtime.test")),
            ("enabled", Value::Bool(true)),
        ],
    );
    logger.flush();
    assert!(stdout.text().is_empty());
    for text in [stderr.text(), read(&plain)] {
        assert!(text.contains(r#"service="gate""#), "{text}");
        assert!(text.contains(r#"component="gate.test""#), "{text}");
        assert!(text.contains(r#"event="runtime.test""#), "{text}");
    }
    let record: serde_json::Value = serde_json::from_str(read(&jsonl).trim()).unwrap();
    assert_eq!(record["service"], "gate");
    assert_eq!(record["component"], "gate.test");
    assert_eq!(record["event"], "runtime.test");
    assert!(logger.stats().iter().all(|s| s.write_errors == 0));
}

/// TestOpenGateLoggerRejectsInvalidConfigBeforeUse.
#[test]
fn gate_rejects_invalid_config_before_use() {
    for (settings, want) in [
        (raw("verbose", "plain", "stderr"), Refusal::Level),
        (raw("info", "yaml", "stderr"), Refusal::ConsoleFormat),
        (raw("info", "plain", "file"), Refusal::ConsoleStream),
    ] {
        assert_eq!(open(Role::Gate, &settings).err(), Some(want));
    }
}

/// Gate keeps Go's trim and case folding of its selectors; Core does not.
#[test]
fn gate_normalizes_selectors_core_does_not() {
    let gate = Settings::validate(Role::Gate, &raw(" INFO ", "JsonL", " stdout")).unwrap();
    assert_eq!(gate.threshold, Some(Level::Info));
    assert_eq!(gate.console, Some((Format::Jsonl, Stream::Stdout)));
    assert_eq!(
        Settings::validate(Role::Core, &raw("INFO", "plain", "stderr")),
        Err(Refusal::Level)
    );
}

/// Stricter than Go Gate: an empty level is refused instead of meaning info.
#[test]
fn empty_level_is_refused_for_both_roles() {
    for role in [Role::Gate, Role::Core] {
        assert_eq!(
            Settings::validate(role, &raw("", "plain", "stderr")),
            Err(Refusal::Level)
        );
    }
}

/// TestOpenGateLoggerRejectsFileOpenFailure.
#[test]
fn gate_rejects_file_open_failure() {
    let dir = TempDir::new("missing");
    let missing = dir.file("missing").join("gate.log");
    let result = open(
        Role::Gate,
        &RawSettings {
            plain_file: path_str(&missing),
            ..raw("info", "off", "stderr")
        },
    );
    assert_eq!(result.err(), Some(Refusal::SinkOpen));
}

/// TestGateFatalClassDoesNotReflectDynamicErrorText.
#[test]
fn fatal_class_does_not_reflect_dynamic_error_text() {
    assert_eq!(fatal_class("proxy: %v"), FatalClass::OriginProxy);
    assert_eq!(fatal_class("proxy: %v").as_str(), "origin_proxy");
    assert_eq!(fatal_class("unknown failure %v"), FatalClass::Startup);
    assert_eq!(fatal_class("unknown failure %v").as_str(), "startup");
    // First match wins in Go order: "admin" precedes "tls".
    assert_eq!(fatal_class("admin tls: %v"), FatalClass::AdminListener);
}

#[test]
fn gate_fatal_record_carries_only_the_static_class() {
    let (mut logger, _, stderr) = open(Role::Gate, &raw("info", "jsonl", "stderr")).unwrap();
    emit_gate_fatal(&mut logger, FIXED, fatal_class("keystore: %v"));
    let record: serde_json::Value = serde_json::from_str(stderr.text().trim()).unwrap();
    assert_eq!(record["level"], "error");
    assert_eq!(record["component"], "gate.runtime");
    assert_eq!(record["event"], "runtime.fatal");
    assert_eq!(record["fields"]["error_class"], "keystore");
    assert_eq!(record["fields"].as_object().unwrap().len(), 1);
}

// --- cmd/server/logging_test.go ---

/// TestLoggingOffByDefault: off emits nothing and observability passes the
/// handler through unwrapped.
#[test]
fn core_logging_off_by_default() {
    assert_eq!(Setting::Level.fallback(Role::Core), "off");
    let (mut logger, stdout, stderr) = open(Role::Core, &raw("off", "plain", "stderr")).unwrap();
    assert!(!logger.is_on());
    assert_eq!(
        observe(&mut logger, FIXED, "GET", HttpVersion::Http1, || 7),
        Completion::Completed(7)
    );
    logger.emit(FIXED, Level::Error, "hidden", &[]);
    assert!(stdout.text().is_empty() && stderr.text().is_empty());
}

/// TestLoggingRejectsInvalidExplicitConfig.
#[test]
fn core_rejects_invalid_explicit_config() {
    for (settings, want) in [
        (raw("", "plain", "stderr"), Refusal::Level),
        (raw("verbose", "plain", "stderr"), Refusal::Level),
        (raw("info", "text", "stderr"), Refusal::ConsoleFormat),
        (raw("info", "plain", "console"), Refusal::ConsoleStream),
    ] {
        assert_eq!(open(Role::Core, &settings).err(), Some(want));
    }
}

/// TestLogSettingPrecedence: flag over environment over fallback.
#[test]
fn setting_precedence() {
    assert_eq!(
        resolve_setting(Some("plain"), Some("jsonl"), "off"),
        "plain"
    );
    assert_eq!(resolve_setting(None, Some("jsonl"), "off"), "jsonl");
    assert_eq!(resolve_setting(None, None, "off"), "off");
    // A present but empty environment value is still a value (LookupEnv).
    assert_eq!(resolve_setting(None, Some(""), "off"), "");
    assert_eq!(Setting::Level.environment(), "HMN_LOG_LEVEL");
    assert_eq!(Setting::JsonlFile.flag(), "log-jsonl-file");
}

/// TestLoggingAccumulatesPlainAndJSONLFiles.
#[test]
fn core_accumulates_plain_and_jsonl_files() {
    let dir = TempDir::new("core");
    let (plain, jsonl) = (dir.file("core.log"), dir.file("core.jsonl"));
    let (mut logger, _, _) = open(
        Role::Core,
        &RawSettings {
            plain_file: path_str(&plain),
            jsonl_file: path_str(&jsonl),
            ..raw("info", "off", "stderr")
        },
    )
    .unwrap();
    logger.emit(
        FIXED,
        Level::Info,
        "Core test event.",
        &[
            ("component", Value::Str("core.test")),
            ("event", Value::Str("test.dual_sink")),
        ],
    );
    logger.flush();
    drop(logger);
    assert!(read(&plain).contains(r#"component="core.test" event="test.dual_sink""#));
    let record: serde_json::Value = serde_json::from_str(&read(&jsonl)).unwrap();
    assert_eq!(record["event"], "test.dual_sink");
    assert_eq!(record["service"], "core");
}

/// TestObservabilityOmitsPathAndPanicMaterial.
#[test]
fn observability_omits_path_and_panic_material() {
    let dir = TempDir::new("observe");
    let jsonl = dir.file("core.jsonl");
    let (mut logger, _, _) = open(
        Role::Core,
        &RawSettings {
            jsonl_file: path_str(&jsonl),
            ..raw("debug", "off", "stderr")
        },
    )
    .unwrap();
    let path_secret = "/path-secret";
    let outcome = observe(&mut logger, FIXED, "GET", HttpVersion::Http1, || {
        let _route = path_secret;
        std::panic::panic_any("panic-secret")
    });
    assert_eq!(outcome, Completion::<()>::PanicRecovered);
    let done = observe(&mut logger, FIXED, "BREW /x", HttpVersion::Http2, || ());
    assert_eq!(done, Completion::Completed(()));
    logger.flush();
    let text = read(&jsonl);
    for forbidden in ["path-secret", "panic-secret", "BREW", "panicked"] {
        assert!(!text.contains(forbidden), "{forbidden} leaked: {text}");
    }
    assert!(text.contains(r#""event":"http.panic_recovered""#), "{text}");
    assert!(text.contains(r#""method":"other""#), "{text}");
    assert!(text.contains(r#""protocol":"2""#), "{text}");
}

/// TestCanonicalExternalInputLabel.
#[test]
fn canonical_external_input_label_bounds() {
    for value in [
        String::new(),
        "external-input/".to_owned(),
        "external-input/run id/profile".to_owned(),
        "external-input/run\n/profile".to_owned(),
        "other/run/profile".to_owned(),
        "x".repeat(193),
    ] {
        assert_eq!(canonical_external_input_label(&value), None, "{value:?}");
    }
    let label = "external-input/run-1/external_input_dom_vusb";
    assert_eq!(canonical_external_input_label(label), Some(label));
    let long = format!("external-input/{}", "a".repeat(192 - 15));
    assert_eq!(long.len(), 192);
    assert!(canonical_external_input_label(&long).is_some());
}

// --- internal/logger record/sink contract used by both roles ---

/// TestPlainAndJSONLShareCanonicalRecord golden lines, byte for byte.
#[test]
fn plain_and_jsonl_share_canonical_record() {
    let settings = Settings::validate(Role::Gate, &raw("debug", "plain", "stderr")).unwrap();
    let (plain, json) = (Shared::default(), Shared::default());
    let mut logger = OperationalLogger::open(
        Identity::new(Role::Gate, "v1.2.3", "gate-1"),
        &settings,
        Box::new(Shared::default()),
        Box::new(plain.clone()),
    )
    .unwrap();
    logger.sinks.push(Sink {
        name: "jsonl",
        format: Format::Jsonl,
        target: Target::Writer(Box::new(json.clone())),
        write_errors: 0,
    });
    logger.emit(
        FIXED,
        Level::Info,
        "Gate listener started.",
        &[
            ("event", Value::Str("runtime.started")),
            ("component", Value::Str("gate.runtime")),
            ("z", Value::Str("last")),
            ("a", Value::I64(1)),
        ],
    );
    assert_eq!(
        plain.text(),
        "2026-07-26T03:34:56.123456789Z INFO service=\"gate\" version=\"v1.2.3\" node=\"gate-1\" component=\"gate.runtime\" event=\"runtime.started\" sequence=1 message=\"Gate listener started.\" field.a=1 field.z=\"last\"\n"
    );
    assert_eq!(
        json.text(),
        "{\"schema_version\":\"1.0.0\",\"ts\":\"2026-07-26T03:34:56.123456789Z\",\"level\":\"info\",\"service\":\"gate\",\"version\":\"v1.2.3\",\"node\":\"gate-1\",\"component\":\"gate.runtime\",\"event\":\"runtime.started\",\"message\":\"Gate listener started.\",\"sequence\":1,\"fields\":{\"a\":1,\"z\":\"last\"}}\n"
    );
}

#[test]
fn timestamps_trim_fraction_and_handle_epoch() {
    assert_eq!(record::rfc3339_nano(Duration::ZERO), "1970-01-01T00:00:00Z");
    assert_eq!(
        record::rfc3339_nano(Duration::new(951_782_400, 500_000_000)),
        "2000-02-29T00:00:00.5Z"
    );
}

#[test]
fn level_threshold_filters() {
    let (mut logger, _, stderr) = open(Role::Core, &raw("warn", "jsonl", "stderr")).unwrap();
    logger.emit(
        FIXED,
        Level::Info,
        "hidden",
        &[("event", Value::Str("runtime.hidden"))],
    );
    logger.emit(
        FIXED,
        Level::Warn,
        "visible",
        &[("event", Value::Str("runtime.visible"))],
    );
    let text = stderr.text();
    assert!(
        !text.contains("hidden") && text.contains("visible"),
        "{text}"
    );
}

/// TestSanitizesHostileTextAndCapsPhysicalLines (no error-valued field can be
/// expressed, so the error case is covered by the type).
#[test]
fn sanitizes_hostile_text_and_caps_physical_lines() {
    let (mut logger, _, stderr) = open(Role::Gate, &raw("info", "plain", "stderr")).unwrap();
    let mut both = Shared::default();
    logger.sinks.push(Sink {
        name: "jsonl",
        format: Format::Jsonl,
        target: Target::Writer(Box::new(both.clone())),
        write_errors: 0,
    });
    let hostile = format!(
        "first\r\n\u{0}\u{1b}[31mred\u{1b}[0m\u{202e}second\u{fffd}{}",
        "x".repeat(9000)
    );
    logger.emit(
        FIXED,
        Level::Info,
        &hostile,
        &[
            ("event", Value::Str("runtime.hostile")),
            ("field\nforged", Value::Str(&hostile)),
            ("bad", Value::F64(f64::NAN)),
        ],
    );
    for line in [stderr.text(), both.text()] {
        assert!(line.len() <= MAX_RECORD_BYTES, "{}", line.len());
        assert_eq!(line.matches('\n').count(), 1);
        for forbidden in ["\r", "\u{0}", "\u{1b}", "\u{202e}", "[31m"] {
            assert!(!line.contains(forbidden), "{forbidden:?}");
        }
        assert!(line.contains("truncated"));
    }
    let _: serde_json::Value = serde_json::from_str(both.text().trim()).unwrap();
    both.flush().unwrap();
}

#[test]
fn invalid_event_and_field_cap_mark_truncation() {
    let (mut logger, _, stderr) = open(Role::Core, &raw("info", "jsonl", "stderr")).unwrap();
    let keys: Vec<String> = (0..MAX_FIELDS + 5).map(|i| format!("k{i:03}")).collect();
    let mut attrs: Vec<(&str, Value<'_>)> =
        keys.iter().map(|k| (k.as_str(), Value::U64(1))).collect();
    attrs.push(("event", Value::Str("Bad..Event")));
    logger.emit(FIXED, Level::Info, "m", &attrs);
    let record: serde_json::Value = serde_json::from_str(stderr.text().trim()).unwrap();
    assert_eq!(record["event"], "runtime.invalid_event");
    assert_eq!(record["fields"].as_object().unwrap().len(), MAX_FIELDS);
    assert_eq!(record["truncated"], true);
}

/// TestFileSinkAppendsOnReopen.
#[test]
fn file_sink_appends_on_reopen() {
    let dir = TempDir::new("append");
    let path = dir.file("core.jsonl");
    for message in ["one", "two"] {
        let (mut logger, _, _) = open(
            Role::Core,
            &RawSettings {
                jsonl_file: path_str(&path),
                ..raw("info", "off", "stderr")
            },
        )
        .unwrap();
        logger.emit(FIXED, Level::Info, message, &[]);
        logger.flush();
    }
    assert_eq!(read(&path).lines().count(), 2);
}

/// TestFileSinkRejectsSamePathAndNonRegularTargets.
#[test]
fn file_sink_rejects_same_path_and_non_regular_targets() {
    let dir = TempDir::new("same");
    let path = dir.file("same.log");
    let same = RawSettings {
        plain_file: path_str(&path),
        jsonl_file: path_str(&path),
        ..raw("info", "off", "stderr")
    };
    assert_eq!(open(Role::Core, &same).err(), Some(Refusal::DuplicateSink));
    let directory = RawSettings {
        plain_file: path_str(&dir.0),
        ..raw("info", "off", "stderr")
    };
    assert_eq!(
        open(Role::Core, &directory).err(),
        Some(Refusal::SinkTarget)
    );
}

/// TestFileSinkRejectsSymlink.
#[cfg(unix)]
#[test]
fn file_sink_rejects_symlink() {
    let dir = TempDir::new("link");
    let target = dir.file("target.log");
    std::fs::write(&target, b"").unwrap();
    let link = dir.file("link.log");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let settings = RawSettings {
        plain_file: path_str(&link),
        ..raw("info", "off", "stderr")
    };
    assert_eq!(open(Role::Core, &settings).err(), Some(Refusal::SinkTarget));
}

/// Stricter than Go: relative and `..` paths are refused, not resolved.
#[test]
fn relative_and_parent_paths_are_refused() {
    for path in ["core.log", "./core.log", "/tmp/../core.log", "   "] {
        let settings = RawSettings {
            plain_file: path,
            ..raw("info", "off", "stderr")
        };
        assert_eq!(
            Settings::validate(Role::Core, &settings),
            Err(Refusal::SinkPath),
            "{path}"
        );
    }
}

/// TestOneFailedSinkDoesNotSuppressAnother.
#[test]
fn one_failed_sink_does_not_suppress_another() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let (mut logger, _, _) = open(Role::Core, &raw("info", "off", "stderr")).unwrap();
    let good = Shared::default();
    for (name, writer) in [
        ("broken", Box::new(Broken) as Box<dyn Write + Send>),
        ("good", Box::new(good.clone())),
    ] {
        logger.sinks.push(Sink {
            name,
            format: Format::Jsonl,
            target: Target::Writer(writer),
            write_errors: 0,
        });
    }
    logger.emit(FIXED, Level::Info, "still written", &[]);
    assert!(good.text().contains("still written"));
    let stats = logger.stats();
    assert_eq!(stats[0].write_errors, 1);
    assert_eq!(stats[1].write_errors, 0);
}

#[test]
fn identity_defaults_unknown_and_sanitizes() {
    let identity = Identity::new(Role::Core, "", "node\u{1b}[31m\n");
    assert_eq!(identity.version, "unknown");
    assert_eq!(identity.node, "node");
    assert_eq!(Refusal::SinkOpen.code(), "log_sink_open");
}

// --- runtime binary `logging` object (cmd/gate main.go `-log-*` flags) ---

/// An empty `logging` object takes the Go Gate flag defaults; unknown fields
/// fail the document.
#[test]
fn inherited_logging_defaults_follow_the_go_gate_flags() {
    let parsed: InheritedLogging = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed, InheritedLogging::default());
    assert_eq!(
        (
            parsed.level.as_str(),
            parsed.console_format.as_str(),
            parsed.console_stream.as_str(),
            parsed.plain_file.as_str(),
            parsed.jsonl_file.as_str(),
        ),
        ("info", "plain", "stderr", "", "")
    );
    assert!(serde_json::from_str::<InheritedLogging>(r#"{"log_level":"info"}"#).is_err());
}

/// Invalid selectors and a standard-output console (the readiness pipe) are
/// refused before any sink opens; with the console off, stdout writes nothing.
#[test]
fn inherited_logging_refuses_the_readiness_pipe_and_invalid_selectors() {
    let open = |config: InheritedLogging| {
        config
            .open("v-test", "node-test", Box::new(Shared::default()))
            .err()
    };
    let stdout = InheritedLogging {
        console_stream: "stdout".into(),
        ..InheritedLogging::default()
    };
    assert_eq!(open(stdout.clone()), Some(Refusal::ConsoleStreamReserved));
    assert_eq!(
        Refusal::ConsoleStreamReserved.code(),
        "log_console_stream_reserved"
    );
    assert_eq!(
        open(InheritedLogging {
            console_format: "off".into(),
            ..stdout
        }),
        None
    );
    assert_eq!(
        open(InheritedLogging {
            level: String::new(),
            ..InheritedLogging::default()
        }),
        Some(Refusal::Level)
    );
    assert_eq!(
        open(InheritedLogging {
            jsonl_file: "relative.jsonl".into(),
            ..InheritedLogging::default()
        }),
        Some(Refusal::SinkPath)
    );
}

/// Go's "Operational logging initialized." record: the console and the file
/// sink carry the same record, which reports file sinks as flags, not paths.
#[test]
fn inherited_logging_writes_the_go_configured_record_without_paths() {
    let dir = TempDir::new("inherited");
    let file = dir.file("gate.jsonl");
    let config = InheritedLogging {
        console_format: "jsonl".into(),
        jsonl_file: path_str(&file).into(),
        ..InheritedLogging::default()
    };
    let stderr = Shared::default();
    let mut logger = config
        .open("v-test", "node-test", Box::new(stderr.clone()))
        .unwrap();
    config.emit_configured(&mut logger, FIXED);
    let record: serde_json::Value = serde_json::from_str(stderr.text().trim()).unwrap();
    assert_eq!(record["service"], "gate");
    assert_eq!(record["level"], "info");
    assert_eq!(record["component"], "gate.runtime");
    assert_eq!(record["event"], "runtime.logging_configured");
    let fields = &record["fields"];
    assert_eq!(fields["console_format"], "jsonl");
    assert_eq!(fields["console_stream"], "stderr");
    assert_eq!(fields["plain_file_enabled"], false);
    assert_eq!(fields["jsonl_file_enabled"], true);
    assert!(!stderr.text().contains(path_str(&file)));
    assert_eq!(read(&file).trim(), stderr.text().trim());
}
