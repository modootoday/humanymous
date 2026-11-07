//! Private L1 planning only. No result grants I/O, ownership or readiness.
//! Native authentication/durability and L3 effect gates remain separate.
use super::limits::{Control, Limits, invalid, resource};
use crate::storage::TailRecovery;
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, PartialEq, Eq)]
enum TailDisposition {
    Covered,
    PreserveReadOnly { first: u64, last: u64 },
}
fn tail(durable: u64, covered: u64, choice: TailRecovery) -> io::Result<TailDisposition> {
    let count = durable
        .checked_sub(covered)
        .ok_or_else(|| invalid("external head exceeds durable records"))?;
    if count == 0 {
        return Ok(TailDisposition::Covered);
    }
    if choice == TailRecovery::RequireCovered {
        return Err(invalid(
            "complete unwitnessed tail requires explicit preservation",
        ));
    }
    Ok(TailDisposition::PreserveReadOnly {
        first: covered
            .checked_add(1)
            .ok_or_else(|| resource("tail index overflow"))?,
        last: durable,
    })
}

#[derive(Clone, Copy)]
enum NormalIntent {
    AppendRecord,
    Checkpoint,
    Ready,
}
fn tail_precondition(disposition: &TailDisposition, intent: NormalIntent) -> io::Result<()> {
    // Ok means this ONE local precondition only, never a successful source or
    // effect capability. No action can mutate/clear a preserved disposition.
    match (disposition, intent) {
        (TailDisposition::Covered, _) => Ok(()),
        (
            TailDisposition::PreserveReadOnly { .. },
            NormalIntent::AppendRecord | NormalIntent::Checkpoint | NormalIntent::Ready,
        ) => Err(invalid("preserved audit tail is read-only")),
    }
}

#[derive(Clone, Copy)]
enum Placement {
    Current,
    Rotate,
}
#[derive(Debug, PartialEq, Eq)]
enum SegmentPlan {
    Current(usize),
    Next(usize),
}
fn append_plan(
    disposition: &TailDisposition,
    placement: Placement,
    segments: usize,
    maximum: usize,
) -> io::Result<SegmentPlan> {
    tail_precondition(disposition, NormalIntent::AppendRecord)?;
    if segments == 0 || maximum == 0 || maximum == usize::MAX || segments > maximum {
        return Err(resource("invalid segment transition bounds"));
    }
    match placement {
        Placement::Current => Ok(SegmentPlan::Current(segments)),
        Placement::Rotate if segments < maximum => Ok(SegmentPlan::Next(
            segments
                .checked_add(1)
                .ok_or_else(|| resource("segment index overflow"))?,
        )),
        Placement::Rotate => Err(resource("segment count exhausted before write")),
    }
}

#[derive(Clone, Copy)]
enum Inventory {
    Empty,
    ControlOnly,
    EmptySegmentWithControl,
    HistoryWithControl,
    HistoryMissingControl,
    InvalidControlOrUnknownMember,
}
#[derive(Clone, Copy)]
enum ExternalAnchor {
    Absent,
    Present,
}
#[derive(Debug, PartialEq, Eq)]
enum BootstrapPlan {
    CreateControl,
    ResumeControl,
    VerifyEmptySegment,
    VerifyExisting,
}
fn bootstrap(
    inventory: Inventory,
    allow_new: bool,
    anchor: ExternalAnchor,
) -> io::Result<BootstrapPlan> {
    // These are observed-shape decisions, not filesystem identity/persistence
    // attestations. A native stage must freshly authenticate/sync the same owner.
    match inventory {
        Inventory::HistoryWithControl => return Ok(BootstrapPlan::VerifyExisting),
        Inventory::HistoryMissingControl | Inventory::InvalidControlOrUnknownMember => {
            return Err(invalid(
                "bootstrap cannot repair missing or invalid control",
            ));
        }
        _ => {}
    }
    if !allow_new || matches!(anchor, ExternalAnchor::Present) {
        return Err(invalid("unexpected empty audit history"));
    }
    match inventory {
        Inventory::Empty => Ok(BootstrapPlan::CreateControl),
        Inventory::ControlOnly => Ok(BootstrapPlan::ResumeControl),
        Inventory::EmptySegmentWithControl => Ok(BootstrapPlan::VerifyEmptySegment),
        _ => unreachable!(),
    }
}

struct OperationBounds<'op, 'cancel> {
    limits: &'op Limits,
    control: &'op Control<'cancel>,
    refused: AtomicBool,
}
impl<'op, 'cancel> OperationBounds<'op, 'cancel> {
    fn capture(limits: &'op Limits, control: &'op Control<'cancel>) -> io::Result<Self> {
        let bounds = Self {
            limits,
            control,
            refused: AtomicBool::new(false),
        };
        bounds.recheck()?;
        Ok(bounds)
    }
    fn recheck(&self) -> io::Result<()> {
        if self.refused.load(Ordering::Acquire) {
            return Err(invalid("recovery operation already refused"));
        }
        let result = self.limits.validate().and_then(|()| self.control.check());
        if result.is_err() {
            self.refused.store(true, Ordering::Release);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn preserved_tail_refuses_every_normal_intent_without_clearing_uncertainty() {
        let disposition = tail(9, 5, TailRecovery::PreserveAmbiguousTail).unwrap();
        assert_eq!(
            disposition,
            TailDisposition::PreserveReadOnly { first: 6, last: 9 }
        );
        for intent in [
            NormalIntent::AppendRecord,
            NormalIntent::Checkpoint,
            NormalIntent::Ready,
        ] {
            assert!(tail_precondition(&disposition, intent).is_err());
        }
        assert!(append_plan(&disposition, Placement::Current, 1, 4096).is_err());
        assert_eq!(
            disposition,
            TailDisposition::PreserveReadOnly { first: 6, last: 9 }
        );
    }

    #[test]
    fn tail_zero_maximum_and_external_ahead_boundaries_are_checked() {
        assert_eq!(
            tail(0, 0, TailRecovery::RequireCovered).unwrap(),
            TailDisposition::Covered
        );
        assert_eq!(
            tail(u64::MAX, u64::MAX, TailRecovery::RequireCovered).unwrap(),
            TailDisposition::Covered
        );
        assert_eq!(
            tail(u64::MAX, u64::MAX - 1, TailRecovery::PreserveAmbiguousTail).unwrap(),
            TailDisposition::PreserveReadOnly {
                first: u64::MAX,
                last: u64::MAX
            }
        );
        assert!(tail(8, 9, TailRecovery::PreserveAmbiguousTail).is_err());
        assert!(tail(9, 8, TailRecovery::RequireCovered).is_err());
    }

    #[test]
    fn rotation_refuses_at_maximum_before_producing_a_new_index() {
        let disposition = tail(7, 7, TailRecovery::RequireCovered).unwrap();
        assert_eq!(
            append_plan(&disposition, Placement::Current, 4, 4).unwrap(),
            SegmentPlan::Current(4)
        );
        assert_eq!(
            append_plan(&disposition, Placement::Rotate, 3, 4).unwrap(),
            SegmentPlan::Next(4)
        );
        assert_eq!(
            append_plan(&disposition, Placement::Rotate, 4, 4)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::FileTooLarge
        );
        for (segments, maximum) in [(0, 4), (5, 4), (1, 0), (usize::MAX, usize::MAX)] {
            assert!(append_plan(&disposition, Placement::Rotate, segments, maximum).is_err());
        }
    }

    #[test]
    fn control_only_restart_requires_explicit_new_history_and_no_anchor() {
        assert_eq!(
            bootstrap(Inventory::ControlOnly, true, ExternalAnchor::Absent).unwrap(),
            BootstrapPlan::ResumeControl
        );
        assert_eq!(
            bootstrap(Inventory::Empty, true, ExternalAnchor::Absent).unwrap(),
            BootstrapPlan::CreateControl
        );
        assert_eq!(
            bootstrap(
                Inventory::EmptySegmentWithControl,
                true,
                ExternalAnchor::Absent
            )
            .unwrap(),
            BootstrapPlan::VerifyEmptySegment
        );
        assert_eq!(
            bootstrap(
                Inventory::HistoryWithControl,
                false,
                ExternalAnchor::Present
            )
            .unwrap(),
            BootstrapPlan::VerifyExisting
        );
        for inventory in [
            Inventory::Empty,
            Inventory::ControlOnly,
            Inventory::EmptySegmentWithControl,
        ] {
            assert!(bootstrap(inventory, false, ExternalAnchor::Absent).is_err());
            assert!(bootstrap(inventory, true, ExternalAnchor::Present).is_err());
        }
        for inventory in [
            Inventory::HistoryMissingControl,
            Inventory::InvalidControlOrUnknownMember,
        ] {
            assert!(bootstrap(inventory, true, ExternalAnchor::Absent).is_err());
        }
    }

    #[test]
    fn original_control_refusal_cannot_be_replaced_or_reset_by_a_fresh_control() {
        let profile = crate::verification::scan::tests::limits();
        let original_cancel = AtomicBool::new(false);
        let original = Control::new(&original_cancel, Duration::from_secs(30)).unwrap();
        let bounds = OperationBounds::capture(&profile, &original).unwrap();
        let fresh_cancel = AtomicBool::new(false);
        let fresh = Control::new(&fresh_cancel, Duration::from_secs(30)).unwrap();
        original_cancel.store(true, Ordering::Release);
        assert!(fresh.check().is_ok());
        assert_eq!(
            bounds.recheck().err().unwrap().kind(),
            io::ErrorKind::Interrupted
        );
        original_cancel.store(false, Ordering::Release);
        assert_eq!(
            bounds.recheck().err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
