//! Native-independent retention tests; no production capability is constructed.
use super::*;
use std::sync::{Arc, Mutex};
type Events = Arc<Mutex<Vec<&'static str>>>;

struct Resource(Events);
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.lock().unwrap().push("resource_drop");
    }
}
struct Guard {
    events: Events,
    cancelled: bool,
}
impl Guard {
    fn cancel(mut self) -> Self {
        if !self.cancelled {
            self.events.lock().unwrap().push("cancel");
        }
        self.cancelled = true;
        self
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.events.lock().unwrap().push(if self.cancelled {
            "cancelled_guard_drop"
        } else {
            "owner_drop_revoke"
        });
    }
}
fn owner() -> (Retained<Resource, Guard>, Events) {
    let events = Arc::new(Mutex::new(Vec::new()));
    (
        Retained::new(
            Resource(events.clone()),
            Guard {
                events: events.clone(),
                cancelled: false,
            },
        ),
        events,
    )
}

#[test]
fn failure_retains_same_resource_allocation_until_owner_release() {
    let (owner, events) = owner();
    let address = std::ptr::from_ref(owner.resources());
    let failure = owner.fail(Guard::cancel);
    assert_eq!(address, std::ptr::from_ref(failure.resources()));
    assert_eq!(*events.lock().unwrap(), ["cancel"]);
    drop(failure);
    assert_eq!(
        *events.lock().unwrap(),
        ["cancel", "cancelled_guard_drop", "resource_drop"]
    );
}
#[test]
fn abandoned_owner_revokes_guard_before_releasing_resources() {
    let (owner, events) = owner();
    drop(owner);
    assert_eq!(
        *events.lock().unwrap(),
        ["owner_drop_revoke", "resource_drop"]
    );
}
#[test]
fn repeated_failure_transition_cannot_release_or_replace_resources() {
    let (owner, events) = owner();
    let address = std::ptr::from_ref(owner.resources());
    let failure = owner.fail(Guard::cancel);
    let failure = failure.retained.fail(Guard::cancel);
    assert_eq!(address, std::ptr::from_ref(failure.resources()));
    assert_eq!(*events.lock().unwrap(), ["cancel"]);
    drop(failure);
    assert_eq!(
        *events.lock().unwrap(),
        ["cancel", "cancelled_guard_drop", "resource_drop"]
    );
}
