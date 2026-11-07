use super::*;

fn same_owner_and_original_bounds<'op, 'cancel>(
    owner: OpeningOwner,
    limits: &'op Limits,
    control: &'op Control<'cancel>,
) -> io::Result<ReadOnlyOwner> {
    let verified = owner.authenticate(limits, control)?;
    verified.finalize()
}
