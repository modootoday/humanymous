use super::*;

fn rejects_clone(verified: VerifiedScan<'_, '_>) {
    let _copy = verified.clone();
}
