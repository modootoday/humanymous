use super::*;
fn baseline() -> NativeBaseline {
    NativeBaseline {
        fetch: true,
        xhr: true,
        stringify: true,
        parse: true,
        console_log: true,
    }
}
fn counts() -> PrototypeCounts {
    PrototypeCounts::new(12, 38).unwrap()
}
#[test]
fn coverage_is_required_and_counter_categories_do_not_mix() {
    let mut epoch = Epoch::new(counts(), baseline());
    assert!(!epoch.snapshot(counts()).unwrap().reported);
    for hook in [Hook::Eval, Hook::Function] {
        epoch.cover(hook);
        epoch.record(hook).unwrap();
    }
    assert!(!epoch.snapshot(counts()).unwrap().reported);
    epoch.cover(Hook::Script);
    epoch.record(Hook::Script).unwrap();
    epoch.record(Hook::Script).unwrap();
    let report = epoch.snapshot(counts()).unwrap();
    assert!(report.reported);
    assert_eq!(
        (report.eval_used, report.func_ctor, report.script_injected),
        (1, 1, 2)
    );
    assert!(!report.native_hooked && !report.console_disabled && !report.proto_polluted);
}
#[test]
fn every_native_failure_and_read_time_prototype_comparison_preserve_original_logic() {
    for mask in 0..32 {
        let native = NativeBaseline {
            fetch: mask & 1 != 0,
            xhr: mask & 2 != 0,
            stringify: mask & 4 != 0,
            parse: mask & 8 != 0,
            console_log: mask & 16 != 0,
        };
        let epoch = Epoch::new(counts(), native);
        let report = epoch.snapshot(counts()).unwrap();
        assert_eq!(report.native_hooked, mask & 15 != 15);
        assert_eq!(report.console_disabled, mask & 16 == 0);
        assert!(
            epoch
                .snapshot(PrototypeCounts::new(13, 38).unwrap())
                .unwrap()
                .proto_polluted
        );
        assert!(!epoch.snapshot(counts()).unwrap().proto_polluted);
    }
    assert!(PrototypeCounts::new(16384, 16384).is_ok());
    assert!(PrototypeCounts::new(16385, 0).is_err());
}
#[test]
fn stop_freezes_and_new_epoch_does_not_inherit_counts_or_coverage() {
    let mut epoch = Epoch::new(counts(), baseline());
    epoch.cover(Hook::Eval);
    epoch.record(Hook::Eval).unwrap();
    let frozen = epoch.stop(counts()).unwrap();
    epoch.cover(Hook::Function);
    epoch.record(Hook::Script).unwrap();
    assert_eq!(
        epoch.snapshot(PrototypeCounts::new(0, 0).unwrap()).unwrap(),
        frozen
    );
    let fresh = Epoch::new(counts(), baseline()).snapshot(counts()).unwrap();
    assert!(!fresh.reported);
    assert_eq!(fresh.eval_used, 0);
}
#[test]
fn counter_overflow_invalidates_every_later_report_instead_of_clamping() {
    let mut epoch = Epoch::new(counts(), baseline());
    epoch.report.eval_used = 9_007_199_254_740_990;
    epoch.record(Hook::Eval).unwrap();
    assert_eq!(
        epoch.snapshot(counts()).unwrap().eval_used,
        9_007_199_254_740_991
    );
    assert!(epoch.record(Hook::Eval).is_err());
    assert!(epoch.snapshot(counts()).is_err());
    assert!(epoch.stop(counts()).is_err());
    assert!(epoch.record(Hook::Script).is_err());
}
