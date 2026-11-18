//! Ports of legacy internal/logger logger_test.go and sink_test.go (baseline
//! 981d25c) against the opt-in queued logger, plus a Go-measured golden for
//! the handler.go attrs/groups nesting. Waits are bounded by generous
//! deadlines; no assertion depends on a sleep or on elapsed time.
use super::super::{
    Attr, Format, Identity, Level, OperationalLogger, RawSettings, Role, Settings, Sink, Target,
    Value,
};
use super::*;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

/// 2026-07-26T03:34:56.123456789Z, the Go logger_test fixed time.
const FIXED: Duration = Duration::new(1_785_036_896, 123_456_789);
/// Generous bound for waits that must succeed.
const PATIENCE: Duration = Duration::from_secs(30);

fn patient() -> Instant {
    Instant::now() + PATIENCE
}

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        lock(&self.0).extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Buffer {
    fn text(&self) -> String {
        String::from_utf8(lock(&self.0).clone()).unwrap()
    }
}

struct Failing;
impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("write failure"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Go `blockingWriter`: signals on its first write, then blocks until the
/// release sender is used or dropped.
struct Blocking {
    entered: Option<mpsc::Sender<()>>,
    release: mpsc::Receiver<()>,
    released: bool,
}
impl Write for Blocking {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(entered) = self.entered.take() {
            let _ = entered.send(());
        }
        if !self.released {
            let _ = self.release.recv();
            self.released = true;
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn blocking() -> (Blocking, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let writer = Blocking {
        entered: Some(entered_tx),
        release: release_rx,
        released: false,
    };
    (writer, entered_rx, release_tx)
}

fn base(level: &str) -> OperationalLogger {
    let settings = Settings::validate(
        Role::Gate,
        &RawSettings {
            level,
            console_format: "off",
            console_stream: "stderr",
            plain_file: "",
            jsonl_file: "",
        },
    )
    .unwrap();
    OperationalLogger::open(
        Identity::new(Role::Gate, "v1.2.3", "gate-1"),
        &settings,
        Box::new(std::io::sink()),
        Box::new(std::io::sink()),
    )
    .unwrap()
}

fn queued(
    level: &str,
    queue_size: usize,
    sinks: Vec<(&'static str, Format, Box<dyn Write + Send>)>,
) -> QueuedLogger {
    let mut logger = base(level);
    for (name, format, writer) in sinks {
        logger.sinks.push(Sink {
            name,
            format,
            target: Target::Writer(writer),
            write_errors: 0,
        });
    }
    QueuedLogger::start(logger, queue_size).unwrap()
}

fn text(key: &str, value: &str) -> Attr {
    Attr::new(key, Value::Str(value))
}

/// TestPlainAndJSONLShareCanonicalRecord through the queue: the bytes match
/// the synchronous golden.
#[test]
fn queued_plain_and_jsonl_share_canonical_record() {
    let (plain, json) = (Buffer::default(), Buffer::default());
    let logger = queued(
        "debug",
        0,
        vec![
            ("plain", Format::Plain, Box::new(plain.clone())),
            ("jsonl", Format::Jsonl, Box::new(json.clone())),
        ],
    );
    logger.handle().emit(
        FIXED,
        Level::Info,
        "Gate listener started.",
        &[
            text("event", "runtime.started"),
            text("component", "gate.runtime"),
            text("z", "last"),
            Attr::new("a", Value::I64(1)),
        ],
    );
    logger.flush(patient()).unwrap();
    assert_eq!(
        plain.text(),
        "2026-07-26T03:34:56.123456789Z INFO service=\"gate\" version=\"v1.2.3\" node=\"gate-1\" component=\"gate.runtime\" event=\"runtime.started\" sequence=1 message=\"Gate listener started.\" field.a=1 field.z=\"last\"\n"
    );
    assert_eq!(
        json.text(),
        "{\"schema_version\":\"1.0.0\",\"ts\":\"2026-07-26T03:34:56.123456789Z\",\"level\":\"info\",\"service\":\"gate\",\"version\":\"v1.2.3\",\"node\":\"gate-1\",\"component\":\"gate.runtime\",\"event\":\"runtime.started\",\"message\":\"Gate listener started.\",\"sequence\":1,\"fields\":{\"a\":1,\"z\":\"last\"}}\n"
    );
}

/// TestLevelValidationAndFiltering through the queue: a filtered record is
/// neither written nor counted as a drop.
#[test]
fn queued_level_threshold_filters_before_enqueue() {
    let output = Buffer::default();
    let logger = queued(
        "warn",
        0,
        vec![("jsonl", Format::Jsonl, Box::new(output.clone()))],
    );
    let handle = logger.handle();
    assert!(!handle.enabled(Level::Info) && handle.enabled(Level::Warn));
    handle.emit(
        FIXED,
        Level::Info,
        "hidden",
        &[text("event", "runtime.hidden")],
    );
    handle.emit(
        FIXED,
        Level::Warn,
        "visible",
        &[text("event", "runtime.visible")],
    );
    logger.flush(patient()).unwrap();
    let written = output.text();
    assert!(
        !written.contains("hidden") && written.contains("visible"),
        "{written}"
    );
    assert_eq!(logger.stats().dropped, DropStats::default());

    let off = queued(
        "off",
        0,
        vec![("jsonl", Format::Jsonl, Box::new(Buffer::default()))],
    );
    off.handle().emit(FIXED, Level::Error, "m", &[]);
    assert_eq!(off.stats().dropped, DropStats::default());
}

#[test]
fn queue_size_zero_selects_default_and_oversize_is_refused() {
    assert_eq!(
        QueuedLogger::start(base("info"), MAX_QUEUE_SIZE + 1).err(),
        Some(QueueRefusal::QueueSize)
    );
    assert_eq!(QueueRefusal::QueueSize.code(), "log_queue_size");
    let logger = QueuedLogger::start(base("info"), 0).unwrap();
    assert_eq!(logger.handle.shared.capacity, DEFAULT_QUEUE_SIZE);
    let logger = QueuedLogger::start(base("info"), MAX_QUEUE_SIZE).unwrap();
    assert_eq!(logger.handle.shared.capacity, MAX_QUEUE_SIZE);
}

/// TestConcurrentProducersProduceIntactJSONLines.
#[test]
fn concurrent_producers_produce_intact_json_lines() {
    let output = Buffer::default();
    let logger = queued(
        "debug",
        2048,
        vec![("jsonl", Format::Jsonl, Box::new(output.clone()))],
    );
    const PRODUCERS: i64 = 8;
    const PER_PRODUCER: i64 = 100;
    thread::scope(|scope| {
        for producer in 0..PRODUCERS {
            let handle = logger.handle();
            scope.spawn(move || {
                for index in 0..PER_PRODUCER {
                    handle.emit(
                        FIXED,
                        Level::Info,
                        "record",
                        &[
                            text("event", "runtime.concurrent"),
                            Attr::new("producer", Value::I64(producer)),
                            Attr::new("index", Value::I64(index)),
                        ],
                    );
                }
            });
        }
    });
    logger.flush(patient()).unwrap();
    let written = output.text();
    let lines: Vec<&str> = written.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        usize::try_from(PRODUCERS * PER_PRODUCER).unwrap(),
        "{:?}",
        logger.stats()
    );
    let mut seen = std::collections::BTreeSet::new();
    for (index, line) in lines.iter().enumerate() {
        let decoded: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(decoded["sequence"], index + 1, "line {index}");
        let fields = &decoded["fields"];
        assert!(seen.insert((fields["producer"].as_i64(), fields["index"].as_i64())));
    }
    assert_eq!(logger.stats().dropped, DropStats::default());
}

/// TestQueueSaturationDoesNotBlockProducer. The writer stays blocked for the
/// whole burst, so completing the burst proves producers do not wait on the
/// sink, and the drop count is exact rather than timing-dependent.
#[test]
fn queue_saturation_does_not_block_producer() {
    let (writer, entered, release) = blocking();
    let logger = queued("info", 1, vec![("jsonl", Format::Jsonl, Box::new(writer))]);
    let handle = logger.handle();
    handle.emit(
        FIXED,
        Level::Info,
        "first",
        &[text("event", "runtime.first")],
    );
    entered
        .recv_timeout(PATIENCE)
        .expect("worker did not enter blocking writer");
    handle.emit(
        FIXED,
        Level::Info,
        "queued",
        &[text("event", "runtime.queued")],
    );

    let (finished_tx, finished) = mpsc::channel();
    let burst = handle.clone();
    let producer = thread::spawn(move || {
        for _ in 0..1000 {
            burst.emit(FIXED, Level::Info, "drop", &[text("event", "runtime.drop")]);
        }
        let _ = finished_tx.send(());
    });
    finished
        .recv_timeout(PATIENCE)
        .expect("producer blocked on a saturated queue");
    producer.join().unwrap();
    let stats = logger.stats();
    assert_eq!(stats.queue_depth, 1);
    assert_eq!(
        stats.dropped.queue_full,
        LevelCounts {
            info: 1000,
            ..LevelCounts::default()
        }
    );
    drop(release);
    logger.close(patient()).unwrap();
}

/// TestOneFailedSinkDoesNotSuppressAnother through the queue.
#[test]
fn queued_failed_sink_does_not_suppress_another() {
    let good = Buffer::default();
    let logger = queued(
        "info",
        0,
        vec![
            ("failed", Format::Jsonl, Box::new(Failing)),
            ("good", Format::Jsonl, Box::new(good.clone())),
        ],
    );
    logger.handle().emit(
        FIXED,
        Level::Info,
        "still delivered",
        &[text("event", "runtime.delivered")],
    );
    logger.flush(patient()).unwrap();
    assert!(good.text().contains("still delivered"));
    let stats = logger.stats();
    assert_eq!(stats.write_errors, 1);
    assert_eq!(
        (stats.sinks[0].write_errors, stats.sinks[1].write_errors),
        (1, 0)
    );
}

/// TestFlushAndCloseAreBoundedAndIdempotent.
#[test]
fn flush_and_close_are_bounded_and_idempotent() {
    let output = Buffer::default();
    let logger = queued(
        "info",
        0,
        vec![("plain", Format::Plain, Box::new(output.clone()))],
    );
    let handle = logger.handle();
    handle.emit(
        FIXED,
        Level::Info,
        "before close",
        &[text("event", "runtime.before_close")],
    );
    logger.flush(patient()).unwrap();
    logger.close(patient()).unwrap();
    logger.close(patient()).unwrap();

    handle.emit(
        FIXED,
        Level::Info,
        "after close",
        &[text("event", "runtime.after_close")],
    );
    assert_eq!(logger.stats().dropped.closed.info, 1);
    logger.flush(patient()).unwrap();
    assert!(output.text().contains("before close"));
    assert!(!output.text().contains("after close"));
}

/// TestFlushHonorsContextWhenSinkIsBlocked, plus the same bound on close.
/// The writer is held until both deadlines have passed, so the timeouts are
/// deterministic; after release the queued record still drains.
#[test]
fn flush_and_close_honor_deadline_when_sink_is_blocked() {
    let (writer, entered, release) = blocking();
    let output = Buffer::default();
    let logger = queued(
        "info",
        0,
        vec![
            ("blocked", Format::Plain, Box::new(writer)),
            ("plain", Format::Plain, Box::new(output.clone())),
        ],
    );
    let handle = logger.handle();
    handle.emit(
        FIXED,
        Level::Info,
        "blocked",
        &[text("event", "runtime.blocked")],
    );
    entered
        .recv_timeout(PATIENCE)
        .expect("worker did not enter blocking writer");
    handle.emit(
        FIXED,
        Level::Info,
        "queued",
        &[text("event", "runtime.queued")],
    );
    let short = Instant::now() + Duration::from_millis(20);
    assert_eq!(logger.flush(short), Err(DeadlineExceeded));
    assert_eq!(logger.close(short), Err(DeadlineExceeded));
    assert_eq!(logger.flush(Instant::now()), Err(DeadlineExceeded));

    drop(release);
    logger.close(patient()).unwrap();
    let written = output.text();
    assert!(
        written.contains("sequence=1") && written.contains("sequence=2"),
        "{written}"
    );
}

/// Go handler.go WithAttrs/WithGroup and slog.Group values. The expected
/// lines were measured by running Go internal/logger at 981d25c with the
/// same calls (`Logger().With(...).WithGroup(...)`, `slog.Group(...)`).
#[test]
fn attrs_and_groups_nest_like_go_handler() {
    let (plain, json) = (Buffer::default(), Buffer::default());
    let mut logger = base("debug");
    logger.identity = Identity::new(Role::Gate, "v1", "n1");
    for (name, format, writer) in [
        ("plain", Format::Plain, plain.clone()),
        ("jsonl", Format::Jsonl, json.clone()),
    ] {
        logger.sinks.push(Sink {
            name,
            format,
            target: Target::Writer(Box::new(writer)),
            write_errors: 0,
        });
    }
    let logger = QueuedLogger::start(logger, 0).unwrap();
    let top = logger
        .handle()
        .with_attrs(&[text("component", "gate.edge"), text("Top Key", "t")]);
    let grouped = top
        .with_group("Req")
        .with_attrs(&[text("component", "nested"), Attr::new("id", Value::I64(7))])
        .with_group("")
        .with_group("inner");
    grouped.emit(
        FIXED,
        Level::Info,
        "grouped",
        &[
            text("event", "x.y"),
            Attr::new("ok", Value::Bool(true)),
            Attr::group(
                "sub",
                vec![
                    Attr::new("a", Value::I64(1)),
                    Attr::group("deep", vec![text("b", "c")]),
                ],
            ),
            text("!!!", "odd"),
        ],
    );
    top.emit(
        FIXED,
        Level::Warn,
        "top",
        &[
            text("event", "runtime.top"),
            Attr::group("", vec![Attr::new("lost", Value::I64(1))]),
            Attr::group("grp", vec![text("component", "inner-comp")]),
        ],
    );
    logger.close(patient()).unwrap();
    assert_eq!(
        plain.text(),
        concat!(
            "2026-07-26T03:34:56.123456789Z INFO service=\"gate\" version=\"v1\" node=\"n1\" component=\"gate.edge\" event=\"runtime.message\" sequence=1 message=\"grouped\" field.req.component=\"nested\" field.req.id=7 field.req.inner.=\"odd\" field.req.inner.event=\"x.y\" field.req.inner.ok=true field.req.inner.sub.a=1 field.req.inner.sub.deep.b=\"c\" field.top_key=\"t\"\n",
            "2026-07-26T03:34:56.123456789Z WARN service=\"gate\" version=\"v1\" node=\"n1\" component=\"gate.edge\" event=\"runtime.top\" sequence=2 message=\"top\" field.grp.component=\"inner-comp\" field.top_key=\"t\" truncated=true\n",
        )
    );
    assert_eq!(
        json.text(),
        concat!(
            "{\"schema_version\":\"1.0.0\",\"ts\":\"2026-07-26T03:34:56.123456789Z\",\"level\":\"info\",\"service\":\"gate\",\"version\":\"v1\",\"node\":\"n1\",\"component\":\"gate.edge\",\"event\":\"runtime.message\",\"message\":\"grouped\",\"sequence\":1,\"fields\":{\"req.component\":\"nested\",\"req.id\":7,\"req.inner.\":\"odd\",\"req.inner.event\":\"x.y\",\"req.inner.ok\":true,\"req.inner.sub.a\":1,\"req.inner.sub.deep.b\":\"c\",\"top_key\":\"t\"}}\n",
            "{\"schema_version\":\"1.0.0\",\"ts\":\"2026-07-26T03:34:56.123456789Z\",\"level\":\"warn\",\"service\":\"gate\",\"version\":\"v1\",\"node\":\"n1\",\"component\":\"gate.edge\",\"event\":\"runtime.top\",\"message\":\"top\",\"sequence\":2,\"fields\":{\"grp.component\":\"inner-comp\",\"top_key\":\"t\"},\"truncated\":true}\n",
        )
    );
}

/// TestFileSinkAppendsOnReopen through the queue: close drains, syncs and
/// closes the file sink, and a second logger appends.
#[test]
fn queued_file_sink_appends_on_reopen_after_close() {
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).unwrap();
    let dir = std::env::temp_dir().join(format!("hmn-logging-queue-{}", hex::encode(nonce)));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("gate.jsonl");
    let path_text = path.to_str().unwrap();
    for message in ["first", "second"] {
        let settings = Settings::validate(
            Role::Gate,
            &RawSettings {
                level: "info",
                console_format: "off",
                console_stream: "stderr",
                plain_file: "",
                jsonl_file: path_text,
            },
        )
        .unwrap();
        let logger = OperationalLogger::open(
            Identity::new(Role::Gate, "v1", "n1"),
            &settings,
            Box::new(std::io::sink()),
            Box::new(std::io::sink()),
        )
        .unwrap();
        let logger = QueuedLogger::start(logger, 0).unwrap();
        logger.handle().emit(
            FIXED,
            Level::Info,
            message,
            &[text("event", "runtime.append")],
        );
        logger.close(patient()).unwrap();
    }
    let content = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = content.trim().split('\n').collect();
    assert_eq!(lines.len(), 2, "{content}");
    assert!(lines[0].contains("first") && lines[1].contains("second"));
}
