//! Actual watchdog and final writer with simulated runtime completion only.
//! The parent must retain unread pipes and independent native cleanup ownership.
use super::{control::Supervisor, invalid, plan::StopPlan, platform::write_report};
use humanymous_runtime::gateway::service::ExitReport;
use serde::Serialize;
use std::{
    io::{self, Write},
    time::Duration,
};

/// Controlled writer-entry observation only, never proof of kernel-blocked I/O.
pub const WRITER_ENTRY: &[u8] = b"HUMANYMOUS-WATCHDOG-WRITER/1\n";

const PRESSURE_BYTES: usize = 8 * 1024 * 1024;
const STOP_MS: u64 = 600;

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    case: &'a str,
    runtime_completion_simulated: bool,
    product_qualified: bool,
    pressure: String,
}

/// Run only in an explicitly owned subprocess with a native retained supervisor.
/// No manager, account, custody, secret provider or network operation is performed.
/// `blocked-*` requires an intentionally unread bounded parent pipe; no runtime
/// drain/flush claim follows from this fixture's synthetic ExitReport.
pub fn run(case: &str) -> io::Result<()> {
    if !matches!(
        case,
        "positive" | "blocked-stdout" | "blocked-stderr" | "late-completion"
    ) {
        return Err(invalid("unsupported watchdog boundary fixture"));
    }
    let (supervisor, _shutdown) = Supervisor::start(
        Duration::from_secs(2),
        StopPlan {
            drain_ms: STOP_MS / 2,
            supervisor_reserve_ms: STOP_MS / 2,
            manager_stop_ms: if cfg!(windows) { 6000 } else { 5000 },
        },
    )?;
    supervisor.ready()?;
    supervisor.published()?;
    supervisor.stop_handle().request()?;
    let completed = ExitReport {
        stopped: true,
        drained: true,
        audit_flushed: true,
    };
    supervisor.deliver_stopped(&completed, || {
        if case == "blocked-stderr" {
            let mut marker = io::stderr().lock();
            marker.write_all(WRITER_ENTRY)?;
            marker.flush()?;
        } else {
            let mut marker = io::stdout().lock();
            marker.write_all(WRITER_ENTRY)?;
            marker.flush()?;
        }
        if case == "late-completion" {
            // Delay this owned writer, not the watchdog. No fresh stop budget.
            std::thread::sleep(Duration::from_millis(STOP_MS * 2));
        }
        let report = Report {
            schema: "humanymous.watchdog-boundary.v1",
            case,
            runtime_completion_simulated: true,
            product_qualified: false,
            pressure: if case.starts_with("blocked-") {
                "x".repeat(PRESSURE_BYTES)
            } else {
                String::new()
            },
        };
        // Serialization, write_all and flush are the production foreground path.
        // No helper reader drains the deliberately pressured stream in this child.
        if case == "blocked-stderr" {
            write_report(&mut io::stderr().lock(), &report)
        } else {
            write_report(&mut io::stdout().lock(), &report)
        }
    })
}
