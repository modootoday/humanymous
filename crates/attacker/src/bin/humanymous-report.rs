//! Frozen legacy observation reporting; publication is not a product pass.
use humanymous_attacker::report::legacy::{self, Error};
use std::{ffi::OsString, io::Write, path::PathBuf};

struct Args {
    input: PathBuf,
    output: PathBuf,
    note: String,
}
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Option<Args>, Error> {
    let mut args = args.into_iter();
    let mut result = Args {
        input: "test/e2e/results.json".into(),
        output: "docs/report.html".into(),
        note: String::new(),
    };
    let mut seen = 0u8;
    while let Some(flag) = args.next() {
        let flag = flag.to_str().ok_or(Error::Arguments)?;
        if matches!(flag, "--help" | "-help" | "-h") {
            return if seen == 0 && args.next().is_none() {
                Ok(None)
            } else {
                Err(Error::Arguments)
            };
        }
        let (name, inline) = flag
            .split_once('=')
            .map_or((flag, None), |(name, value)| (name, Some(value)));
        let bit = match name {
            "--in" | "-in" => 1,
            "--out" | "-out" => 2,
            "--note" | "-note" => 4,
            _ => return Err(Error::Arguments),
        };
        if seen & bit != 0 {
            return Err(Error::Arguments);
        }
        seen |= bit;
        let value = match inline {
            Some(value) => OsString::from(value),
            None => args.next().ok_or(Error::Arguments)?,
        };
        if (value.is_empty() && bit != 4) || value.len() > legacy::FIELD_BYTES {
            return Err(Error::Arguments);
        }
        match bit {
            1 => result.input = value.into(),
            2 => result.output = value.into(),
            _ => result.note = value.into_string().map_err(|_| Error::Arguments)?,
        }
    }
    Ok(Some(result))
}
fn run() -> Result<(), Error> {
    let Some(args) = parse(std::env::args_os().skip(1))? else {
        let _ = writeln!(
            std::io::stdout(),
            "humanymous-report [--in PATH] [--out PATH] [--note CAPTION]"
        );
        return Ok(());
    };
    let summary = legacy::generate(&args.input, &args.output, &args.note)?;
    let bot_rate = if summary.bot_runs == 0 {
        "N/A".into()
    } else {
        format!("{:.0}%", summary.bot_tpr * 100.0)
    };
    let human_rate = if summary.human_runs == 0 {
        "N/A".into()
    } else {
        format!("{:.0}%", summary.human_fpr * 100.0)
    };
    // Publication has committed. A broken output pipe must not turn it into a
    // reported filesystem failure or attempt to roll back another file.
    let _ = writeln!(
        std::io::stdout(),
        "report written; declared-label botTPR={bot_rate} humanFPR={human_rate}; eligible bots={} humans={}; scheduled={} errors={} skipped={}; unsupported=not_encoded; not a product pass",
        summary.bot_runs,
        summary.human_runs,
        summary.accounting.scheduled,
        summary.accounting.errors,
        summary.accounting.skipped
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("report: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cli_defaults_and_bounded_unambiguous_flags() {
        let defaults = parse(Vec::<OsString>::new()).ok().flatten().unwrap();
        assert_eq!(defaults.input, PathBuf::from("test/e2e/results.json"));
        assert_eq!(defaults.output, PathBuf::from("docs/report.html"));
        assert!(defaults.note.is_empty());
        assert!(parse([OsString::from("--help")]).ok().unwrap().is_none());
        for args in [
            vec!["--in"],
            vec!["--wat", "x"],
            vec!["--out", "a", "-out=b"],
            vec!["--help", "x"],
        ] {
            assert!(parse(args.into_iter().map(OsString::from)).is_err());
        }
        let args = parse(
            ["-in=a", "--out=b", "--note", "fixed"]
                .into_iter()
                .map(OsString::from),
        )
        .ok()
        .flatten()
        .unwrap();
        assert_eq!(args.note, "fixed");
    }
}
