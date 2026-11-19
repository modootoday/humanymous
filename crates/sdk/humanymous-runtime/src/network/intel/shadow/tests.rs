use super::*;

#[test]
fn counts_only_shadow_categories() {
    let counters = ShadowCounters::default();
    counters.record(&Classification {
        datacenter: true,
        ..Default::default()
    });
    assert_eq!(counters.stats(), ShadowStats::default());
    counters.record(&Classification {
        tor_exit: true,
        ..Default::default()
    });
    counters.record(&Classification {
        datacenter: true,
        tor_exit: true,
        blocklist: true,
    });
    assert_eq!(
        counters.stats(),
        ShadowStats {
            tor_exit_hits: 2,
            blocklist_hits: 1,
            tor_exit_and_blocklist_hits: 1,
        }
    );
}
