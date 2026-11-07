use super::*;

fn rejects_reusing_opening(owner: OpeningOwner, limits: &Limits, control: &Control<'_>) {
    let _first = owner.authenticate(limits, control);
    let _second = owner.authenticate(limits, control);
}
