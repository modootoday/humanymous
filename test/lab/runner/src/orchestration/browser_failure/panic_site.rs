//! Explicit selected-source binding permits fixed panic codes, never raw text.
use crate::snapshot::source_pin::SourceBinding;
mod bootstrap;

fn canonical_number(value: &str) -> Option<usize> {
    if value.is_empty()
        || value.len() > 3
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

fn failure_block<'a>(output: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let header = format!("---- {name} output ----");
    let mut started = false;
    let mut closed = false;
    let mut inside = false;
    let mut replay = false;
    let mut block = Vec::new();
    for line in output.lines() {
        if line == "console output:" {
            if !closed || replay {
                return None;
            }
            replay = true;
        } else if line.starts_with("---- ") && line.ends_with(" output ----") {
            if line != header || started {
                return None;
            }
            started = true;
            inside = true;
        } else if inside && line == "failures:" {
            closed = true;
            inside = false;
        } else if inside {
            block.push(line);
        } else if !replay && line.trim().starts_with("panicked at ") {
            return None;
        }
    }
    (started && closed).then_some(block)
}

pub(super) fn classify(
    output: &str,
    name: &str,
    binding: Option<&SourceBinding>,
) -> Option<&'static str> {
    let binding = binding?;
    if name != binding.name() || !binding.refinement_ready() {
        return None;
    }
    let prefix = format!("panicked at {}:", binding.path());
    let mut found = None;
    let block = failure_block(output, name)?;
    for (index, line) in block.iter().map(|line| line.trim()).enumerate() {
        if !line.starts_with("panicked at ") {
            continue;
        }
        let location = line.strip_prefix(&prefix)?.strip_suffix(':')?;
        let (line_number, column) = location.split_once(':')?;
        let line_number = canonical_number(line_number)?;
        let column = canonical_number(column)?;
        let code = binding.site(line_number, column)?;
        if found.is_some() {
            return None;
        }
        found = Some(if code == "browser_panic_bootstrap" {
            bootstrap::refine(&block, index)?
        } else {
            code
        });
    }
    binding.revalidate().ok()?;
    found
}

#[cfg(test)]
mod tests {
    pub(super) const NAME: &str = "browser::transport::runtime_pilot::actual_runtime_session_report_and_required_capability_fail_closed";
    pub(super) const PATH: &str = "crates/collector/src/browser/transport/runtime_pilot.rs";

    pub(super) fn classify(output: &str, name: &str) -> Option<&'static str> {
        let fixture = crate::snapshot::source_pin::tests::fixture();
        super::classify(output, name, Some(&fixture.binding))
    }

    pub(super) fn report(body: &str, console: &str) -> String {
        format!(
            "test {NAME} ... FAIL\n---- {NAME} output ----\n{body}\nfailures:\n{NAME}\nconsole output:\n{console}"
        )
    }

    #[test]
    fn synthetic_locations_reject_missing_binding_drift_and_payloads() {
        let valid = format!("panicked at {PATH}:22:10:");
        let complete = report(&valid, &valid);
        assert_eq!(classify(&complete, NAME), Some("browser_panic_bootstrap"));
        assert_eq!(super::classify(&complete, NAME, None), None);
        assert_eq!(classify(&complete, "foreign::test"), None);
        let fixture = crate::snapshot::source_pin::tests::fixture();
        fixture.drift();
        assert_eq!(
            super::classify(&complete, NAME, Some(&fixture.binding)),
            None
        );
        for bad in [
            format!("{valid} PRIVATE_BODY"),
            format!("panicked at /private/{PATH}:22:10:"),
            format!("panicked at {PATH}:022:10:"),
            format!("panicked at {PATH}:22:0:"),
            format!("panicked at {PATH}:22:999:"),
            format!("panicked at {PATH}:23:5:"),
            format!("panicked at {PATH}:22:10:extra:"),
            "panicked at PRIVATE_PATH".into(),
        ] {
            assert_eq!(classify(&report(&bad, ""), NAME), None);
            assert_eq!(
                classify(&report(&format!("{valid}\n{bad}"), ""), NAME),
                None
            );
        }
        assert_eq!(
            classify(&report(&format!("{valid}\n{valid}"), ""), NAME),
            None
        );
        assert_eq!(classify(&report("PRIVATE_BODY", &valid), NAME), None);
    }

    #[test]
    fn only_one_closed_owned_failure_block_can_supply_a_location() {
        let valid = format!("panicked at {PATH}:22:10:");
        let complete = report(&valid, &valid);
        assert_eq!(classify(&complete, NAME), Some("browser_panic_bootstrap"));
        for invalid in [
            valid.clone(),
            format!("---- {NAME} output ----\n{valid}"),
            format!("---- {NAME} output ---- FORGED\n{valid}\nfailures:"),
            format!("{complete}\n---- {NAME} output ----\n"),
            format!("{complete}\n---- foreign::test output ----\n"),
            complete.replace("console output:\n", &format!("{valid}\nconsole output:\n")),
            format!("{complete}\nconsole output:\n"),
        ] {
            assert_eq!(classify(&invalid, NAME), None);
        }
    }
}
