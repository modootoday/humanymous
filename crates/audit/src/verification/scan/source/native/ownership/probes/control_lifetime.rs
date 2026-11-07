use super::*;

fn rejects_local_control<'op, 'cancel>(
    owner: OpeningOwner,
    limits: &'op Limits,
    cancel: &'cancel std::sync::atomic::AtomicBool,
) -> io::Result<VerifiedScan<'op, 'cancel>> {
    let control = Control::new(cancel, std::time::Duration::from_secs(30))?;
    owner.authenticate(limits, &control)
}
