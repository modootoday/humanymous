use super::*;

fn rejects_fresh_bounds(verified: VerifiedScan<'_, '_>, limits: &Limits, control: &Control<'_>) {
    let _result = verified.finalize(limits, control);
}
