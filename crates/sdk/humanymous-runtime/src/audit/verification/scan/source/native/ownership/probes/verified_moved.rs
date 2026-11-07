use super::*;

fn rejects_reusing_verified(verified: VerifiedScan<'_, '_>) {
    let _first = verified.finalize();
    let _second = verified.finalize();
}
