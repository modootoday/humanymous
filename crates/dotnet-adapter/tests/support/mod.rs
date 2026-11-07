//! Explicit execution provenance for actual integration tests, never a security token.
pub fn execution_environment() -> &'static str {
    #[cfg(unix)]
    assert_ne!(
        humanymous_native_security::process::current_uid(),
        0,
        "ordinary non-root .NET actor required"
    );
    classify(
        std::env::var("GITHUB_ACTIONS").ok().as_deref(),
        std::env::var("RUNNER_ENVIRONMENT").ok().as_deref(),
        std::env::var_os("ACT").is_some(),
        std::env::var("HUMANYMOUS_DOTNET_LOCAL_ALLOCATION").ok().as_deref(),
    ).expect("actual .NET tests require allocated local or genuine standard hosted execution; ACT is not qualification")
}

fn classify(
    actions: Option<&str>,
    runner: Option<&str>,
    emulated: bool,
    allocation: Option<&str>,
) -> Option<&'static str> {
    if emulated {
        None
    } else if actions == Some("true") && runner == Some("github-hosted") {
        Some("github-hosted")
    } else if allocation == Some("root-approved-local") {
        Some("root-allocated-local")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::classify;
    #[test]
    fn emulation_never_claims_hosted_or_allocated_actual_execution() {
        assert_eq!(
            classify(Some("true"), Some("github-hosted"), false, None),
            Some("github-hosted")
        );
        assert_eq!(
            classify(Some("true"), Some("github-hosted"), true, None),
            None
        );
        assert_eq!(
            classify(None, None, true, Some("root-approved-local")),
            None
        );
        assert_eq!(
            classify(Some("true"), Some("self-hosted"), false, None),
            None
        );
        assert_eq!(
            classify(None, None, false, Some("root-approved-local")),
            Some("root-allocated-local")
        );
        assert_eq!(classify(None, None, false, None), None);
    }
}
