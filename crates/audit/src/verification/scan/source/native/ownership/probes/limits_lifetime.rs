use super::*;

fn rejects_local_limits<'op, 'cancel>(
    owner: OpeningOwner,
    control: &'op Control<'cancel>,
) -> io::Result<VerifiedScan<'op, 'cancel>> {
    let limits = crate::verification::scan::tests::limits();
    owner.authenticate(&limits, control)
}
