//! Conservative compiler detection with command boundaries, not cargo substrings.
pub fn has_go_build(text: &str) -> bool {
    let tokens: Vec<_> = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .flat_map(|line| {
            line.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/')))
        })
        .filter(|token| !token.is_empty())
        .collect();
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(token.rsplit('/').next(), Some("go" | "go.exe"))
            && tokens[index + 1..].contains(&"build")
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cargo_is_not_go_but_real_go_builds_remain_blocked() {
        assert!(!has_go_build(
            "# no go build\nRUN cargo build --release --locked -p humanymous-runtime"
        ));
        for source in [
            "RUN go build ./cmd/gate",
            "RUN go -tags prod build ./cmd/gate",
            "RUN /usr/local/go/bin/go build .",
            "RUN go.exe build .",
        ] {
            assert!(has_go_build(source));
        }
    }
}
