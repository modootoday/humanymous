use super::*;

fn vector(base: f32, jitter: f32) -> FeatureVector {
    let mut out = [0.0; FEATURE_DIM];
    for (dim, slot) in out.iter_mut().enumerate() {
        *slot = base + jitter * ((dim % 3) as f32 - 1.0);
    }
    out
}

fn seeded(store: &mut PatternStore, base: f32, members: u64, now: u64) -> ClusterId {
    let mut id = None;
    for index in 0..members {
        let jitter = 0.02 * (index % 3) as f32;
        id = Some(
            store
                .admit(&vector(base, jitter), Evidence::CatalogBot, now)
                .expect("admit"),
        );
    }
    id.expect("members > 0")
}

#[test]
fn similarity_is_one_at_zero_distance_and_decreasing() {
    assert_eq!(similarity(0.0), 1.0);
    assert!(similarity(0.5) > similarity(1.0));
    assert!(similarity(10.0) < 1e-10);
}

#[test]
fn close_confirmed_vectors_join_one_cluster() {
    let mut store = PatternStore::new();
    let id = seeded(&mut store, 1.0, 6, 10);
    assert_eq!(store.len(), 1);
    assert_eq!(store.views()[0].id, id);
    assert_eq!(store.views()[0].support, 6);
    assert_eq!(store.views()[0].catalog_support, 6);
}

#[test]
fn distant_confirmed_vectors_start_new_clusters() {
    let mut store = PatternStore::new();
    seeded(&mut store, 1.0, 1, 10);
    seeded(&mut store, 5.0, 1, 10);
    assert_eq!(store.len(), 2);
}

#[test]
fn under_supported_cluster_never_matches() {
    let mut store = PatternStore::new();
    seeded(&mut store, 1.0, MIN_SUPPORT - 1, 10);
    assert_eq!(store.nearest(&vector(1.0, 0.0)).expect("probe"), None);
    store
        .admit(&vector(1.0, 0.0), Evidence::OperatorConfirmed, 11)
        .expect("admit");
    let found = store
        .nearest(&vector(1.0, 0.0))
        .expect("probe")
        .expect("match");
    assert!(found.similarity > 0.9);
}

#[test]
fn far_probe_has_low_similarity() {
    let mut store = PatternStore::new();
    seeded(&mut store, 1.0, 6, 10);
    let near = store
        .nearest(&vector(1.0, 0.0))
        .expect("probe")
        .expect("match");
    let far = store
        .nearest(&vector(4.0, 0.0))
        .expect("probe")
        .expect("match");
    assert!(near.similarity > far.similarity);
    assert!(far.similarity < 0.01);
}

#[test]
fn probing_does_not_change_the_store() {
    let mut store = PatternStore::new();
    seeded(&mut store, 1.0, 6, 10);
    let before = store.views();
    for _ in 0..50 {
        store.nearest(&vector(1.3, 0.1)).expect("probe");
    }
    assert_eq!(store.views(), before);
}

#[test]
fn recorded_matches_change_recurrence_not_centroid() {
    let mut store = PatternStore::new();
    let id = seeded(&mut store, 1.0, 6, 10);
    let before = store.nearest(&vector(1.2, 0.0)).expect("probe");
    for at in [100, 200, 300] {
        store.record_match(id, at).expect("record");
    }
    assert_eq!(store.nearest(&vector(1.2, 0.0)).expect("probe"), before);
    assert_eq!(store.recurrence(id, 150, 300).expect("count"), 2);
    assert_eq!(store.recurrence(id, 1000, 300).expect("count"), 3);
}

#[test]
fn match_history_is_bounded() {
    let mut store = PatternStore::new();
    let id = seeded(&mut store, 1.0, 6, 0);
    for at in 0..(MAX_MATCH_EVENTS as u64 + 10) {
        store.record_match(id, at).expect("record");
    }
    assert_eq!(store.views()[0].matches_held, MAX_MATCH_EVENTS);
}

#[test]
fn nonfinite_vector_is_refused_without_state_change() {
    let mut store = PatternStore::new();
    let mut bad = vector(1.0, 0.0);
    bad[3] = f32::NAN;
    assert_eq!(
        store.admit(&bad, Evidence::CatalogBot, 1),
        Err(SimilarityError::Nonfinite)
    );
    assert!(store.is_empty());
    assert_eq!(store.nearest(&bad), Err(SimilarityError::Nonfinite));
}

#[test]
fn clock_regression_is_refused() {
    let mut store = PatternStore::new();
    let id = seeded(&mut store, 1.0, 6, 100);
    assert_eq!(
        store.admit(&vector(1.0, 0.0), Evidence::CatalogBot, 99),
        Err(SimilarityError::ClockRegressed)
    );
    assert_eq!(store.views()[0].support, 6);
    assert_eq!(
        store.record_match(id, 50),
        Err(SimilarityError::ClockRegressed)
    );
    assert_eq!(
        store.recurrence(id, 10, 50),
        Err(SimilarityError::ClockRegressed)
    );
}

#[test]
fn full_store_refuses_instead_of_evicting() {
    let mut store = PatternStore::new();
    for index in 0..MAX_CLUSTERS {
        seeded(&mut store, index as f32 * 10.0, 1, 1);
    }
    assert_eq!(
        store.admit(&vector(-1e6, 0.0), Evidence::CatalogBot, 2),
        Err(SimilarityError::Capacity)
    );
    assert_eq!(store.len(), MAX_CLUSTERS);
}

#[test]
fn stale_clusters_and_old_matches_expire() {
    let mut store = PatternStore::new();
    let old = seeded(&mut store, 1.0, 6, 0);
    let fresh = seeded(&mut store, 9.0, 6, RETENTION_SECS);
    store.record_match(fresh, RETENTION_SECS).expect("record");
    store
        .record_match(fresh, RETENTION_SECS + 500)
        .expect("record");
    let removed = store.expire(RETENTION_SECS + 1000, 600).expect("expire");
    assert_eq!(removed, 1);
    assert_eq!(store.retire(old), Err(SimilarityError::UnknownCluster));
    assert_eq!(store.views()[0].matches_held, 1);
}

#[test]
fn retired_cluster_is_gone_and_ids_are_not_reused() {
    let mut store = PatternStore::new();
    let first = seeded(&mut store, 1.0, 1, 1);
    store.retire(first).expect("retire");
    let second = seeded(&mut store, 1.0, 1, 2);
    assert_ne!(first, second);
    assert_eq!(
        store.record_match(first, 3),
        Err(SimilarityError::UnknownCluster)
    );
}
