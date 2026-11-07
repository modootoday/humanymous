//! External Rust consumer of the deployment surface; no host execution claim.
use humanymous_adapter_go::{Configuration, Deployment, Host, Rejection};

// Keep both directions tied to the owning runtime types without conversion.
const _: fn(
    Host,
    humanymous_runtime::configuration::Configuration,
) -> Result<Deployment, Rejection> = Deployment::new;
const _: fn(Deployment) -> Configuration = Deployment::into_configuration;
const _: fn(Host, Configuration) -> Result<Deployment, humanymous_runtime::Rejection> =
    Deployment::new;

#[test]
fn pooled_context_and_borrowed_entity_are_distinct_host_contracts() {
    for (host, pooled, borrowed) in [
        (Host::NetHttp, false, false),
        (Host::Chi, false, false),
        (Host::Gin, true, false),
        (Host::Echo, true, false),
        (Host::Fiber, true, true),
        (Host::FastHttp, true, true),
    ] {
        assert_eq!(host.pooled_context(), pooled, "{host:?}");
        assert_eq!(host.borrows_entity_slices(), borrowed, "{host:?}");
    }
}
