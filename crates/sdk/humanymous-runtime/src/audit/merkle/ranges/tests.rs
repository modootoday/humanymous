use super::*;

fn assert_scan_order(plan: &Plan) {
    let mut end = 0;
    let mut seen = [false; CAPACITY];
    for (index, range) in plan.visits() {
        assert!(index < plan.len());
        assert!(!seen[index]);
        seen[index] = true;
        assert!(range.size > 0);
        assert!(range.start >= end);
        end = range.start.checked_add(range.size).unwrap();
        assert!(end <= plan.total);
        assert_eq!(range, plan.ranges[index]);
    }
    assert!(seen[..plan.len()].iter().all(|seen| *seen));
}

#[test]
fn proof_ranges_are_disjoint_and_visit_in_file_order() {
    for size in 1..=129 {
        for index in 0..size {
            assert_scan_order(&Plan::inclusion(index, size).unwrap());
        }
        for old in 1..size {
            assert_scan_order(&Plan::consistency(old, size).unwrap());
        }
    }
    assert!(std::mem::size_of::<Plan>() <= 2048);
}

#[test]
fn maximum_counts_fit_fixed_capacity_without_allocating_history() {
    let plan = Plan::consistency((1_u64 << 63) + 1, u64::MAX).unwrap();
    assert_eq!(plan.len(), 65);
    assert_scan_order(&plan);
    for index in [0, 1, 1_u64 << 63, u64::MAX - 1] {
        let plan = Plan::inclusion(index, u64::MAX).unwrap();
        assert!(plan.len() <= 64);
        assert_scan_order(&plan);
    }
    assert!(Plan::inclusion(0, 0).is_none());
    assert!(Plan::inclusion(u64::MAX, u64::MAX).is_none());
    assert!(Plan::consistency(0, u64::MAX).is_none());
    assert!(Plan::consistency(u64::MAX, u64::MAX).is_none());
    assert!(Plan::consistency(u64::MAX, 1).is_none());
}

#[test]
fn invalid_range_or_capacity_refuses_before_mutation() {
    let mut plan = Plan::new(u64::MAX);
    for (start, size) in [(0, 0), (u64::MAX, 1), (u64::MAX - 1, 2)] {
        let before = plan.clone();
        assert_eq!(plan.push(start, size), None);
        assert_eq!(plan, before);
    }
    for _ in 0..CAPACITY {
        plan.push(0, 1).unwrap();
    }
    let before = plan.clone();
    assert_eq!(plan.push(0, 1), None);
    assert_eq!(plan, before);
}
