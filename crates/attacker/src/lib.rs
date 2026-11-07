//! Defense-only deterministic observation replay. No sockets or arbitrary targets.
pub mod catalog;
pub mod network;
pub mod report;

/// Run every scheduled case exactly once against the real default engine.
pub fn run(seed: u64) -> report::RunReport {
    let rows = catalog::cases(seed)
        .into_iter()
        .map(report::execute)
        .collect();
    report::RunReport::new(seed, rows)
}

#[cfg(test)]
mod tests {
    #[test]
    fn seed_reproduces_full_actual_engine_report() {
        let first = serde_json::to_value(super::run(42)).unwrap();
        assert_eq!(first, serde_json::to_value(super::run(42)).unwrap());
        assert_ne!(
            first["rows"],
            serde_json::to_value(super::run(43)).unwrap()["rows"]
        );
    }
}
