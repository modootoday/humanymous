//! Explicit execution provenance for actual integration tests, never a security token.
pub fn execution_environment() -> &'static str {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
    {
        "github-hosted"
    } else if std::env::var("HUMANYMOUS_DOTNET_LOCAL_ALLOCATION").as_deref()
        == Ok("root-approved-local")
    {
        "root-allocated-local"
    } else {
        panic!("actual .NET tests require an explicit root allocation or standard hosted execution")
    }
}
