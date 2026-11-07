//! Registered completion ownership on real CPython Futures; not ASGI admission.
use super::*;

#[derive(Clone, Copy)]
enum Case {
    Late,
    Cancel,
    Wrong,
}

fn scenario(case: Case) {
    assert_eq!(
        std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref(),
        Ok("1")
    );
    Python::initialize();
    Python::attach(|py| -> PyResult<()> {
        // The explicit SDK selector runs serially; no private counter mutation.
        let _occupied: Vec<_> = (0..63).map(|_| Admission::acquire().expect("isolated SDK slots")).collect();
        let asyncio = py.import("asyncio")?;
        let event_loop = asyncio.call_method0("new_event_loop")?;
        let future = event_loop.call_method0("create_future")?;
        let (tx, mut rx) = oneshot::channel();
        let state = Arc::new(Mutex::new(State {
            cancelled: false, started: true, admission: None,
            permit: Some(Admission::acquire().expect("completion owner slot")),
            task: Some(future.clone().unbind()), result: Some(tx),
        }));
        let weak = Arc::downgrade(&state);
        let done = Py::new(py, Done { state: state.clone() })?;
        drop(state);
        if matches!(case, Case::Wrong) {
            let other = event_loop.call_method0("create_future")?;
            other.call_method1("set_result", (0u8,))?;
            assert!(done.bind(py).call1((&other,)).is_err());
            assert!(done.bind(py).call1((&future,)).is_err());
            assert!(Admission::acquire().is_err());
            future.call_method1("set_result", (42u8,))?;
            done.bind(py).call1((&future,))?;
            drop(done);
        } else {
            future.call_method1("add_done_callback", (done,))?;
            assert!(weak.upgrade().is_some(), "callback retains owner after observer drop");
            assert!(Admission::acquire().is_err());
            if matches!(case, Case::Cancel) {
                assert!(future.call_method0("cancel")?.extract::<bool>()?);
                assert!(Admission::acquire().is_err(), "cancel request is not callback settlement");
            } else {
                event_loop.call_method1("call_later", (0.01, future.getattr("set_result")?, 42u8))?;
            }
            let turn = asyncio.call_method1("sleep", (0.02,))?;
            event_loop.call_method1("run_until_complete", (turn,))?;
        }
        let result = rx.try_recv().expect("actual terminal callback delivered");
        if matches!(case, Case::Cancel) {
            assert!(result.is_err());
            assert!(future.call_method0("cancelled")?.extract::<bool>()?);
        } else {
            assert_eq!(result?.bind(py).extract::<u8>()?, 42);
        }
        assert!(future.call_method0("done")?.extract::<bool>()?);
        assert!(weak.upgrade().is_none(), "terminal callback breaks retained cycle");
        assert!(Admission::acquire().is_ok(), "terminal observation releases one slot");
        event_loop.call_method0("close")?;
        println!("python-completion registered=true terminal=true permit_released=true authority_qualified=false");
        Ok(())
    }).unwrap();
}

#[test]
#[ignore = "allocated actual CPython ownership control; --test-threads=1 required"]
fn late_completion_retains_owner_and_permit_after_observer_drop() {
    scenario(Case::Late);
}
#[test]
#[ignore = "allocated actual CPython ownership control; --test-threads=1 required"]
fn cancellation_waits_for_registered_terminal_observation() {
    scenario(Case::Cancel);
}
#[test]
#[ignore = "allocated actual CPython ownership control; --test-threads=1 required"]
fn wrong_or_nonterminal_callback_cannot_release_permit() {
    scenario(Case::Wrong);
}
