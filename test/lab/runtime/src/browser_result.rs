//! Exact accounting of the unchanged upstream wasm test runner text protocol.
pub fn completed(output: &str, name: &str, filtered: usize) -> bool {
    let lines: Vec<_> = output.lines().map(str::trim).collect();
    let summaries: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    let expected =
        format!("test result: ok. 1 passed; 0 failed; 0 ignored; {filtered} filtered out");
    let summary_ok = summaries.len() == 1
        && (summaries[0] == expected.as_str()
            || summaries[0]
                .strip_prefix(expected.as_str())
                .is_some_and(|tail| tail.starts_with("; finished in ")));
    let test = format!("test {name} ... ok");
    summary_ok
        && lines
            .iter()
            .filter(|line| **line == "running 1 test")
            .count()
            == 1
        && lines.iter().filter(|line| **line == test).count() == 1
        && lines
            .iter()
            .filter(|line| line.starts_with("test ") && !line.starts_with("test result:"))
            .count()
            == 1
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn summary_substrings_and_wrong_denominators_cannot_succeed() {
        let good = "running 1 test\ntest owned::join ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 66 filtered out; finished in 0.2s\n";
        assert!(completed(good, "owned::join", 66));
        for bad in [
            good.replace("1 passed", "11 passed"),
            good.replace("0 failed", "1 failed"),
            good.replace("0 ignored", "1 ignored"),
            good.replace("66 filtered", "65 filtered"),
            good.replace("owned::join", "owned::other"),
            format!("{good}{good}"),
            good.replace("test result:", "quoted test result:"),
        ] {
            assert!(!completed(&bad, "owned::join", 66));
        }
    }
}
