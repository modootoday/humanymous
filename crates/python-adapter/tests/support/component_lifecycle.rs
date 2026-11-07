//! One explicit execution refusal until the shared owned-lifetime API is reviewed.
pub(super) fn require_safe_lifecycle() -> Result<(), String> {
    Err("NOT_ACCEPTED: shared atomic owned-lifetime contract is not available".into())
}
