use super::SessionReport;
use crate::observation::Signal;

impl SessionReport {
    /// Copy client observations followed by network observations, preserving order.
    pub fn all_signals(&self) -> Vec<Signal> {
        self.client
            .signals
            .iter()
            .flatten()
            .chain(self.network.signals.iter().flatten())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{Source, Verdict};

    #[test]
    fn merged_observations_preserve_plane_order_and_are_independent() {
        let mut report = SessionReport::default();
        report.client.signals = Some(vec![Signal::new(
            "l1.navigator.webdriver",
            serde_json::json!(true),
            Verdict::Bot,
            1.0,
            Source::Wasm,
            "",
        )]);
        report.network.signals = Some(vec![Signal::new(
            "l5.rit.ok",
            serde_json::json!(true),
            Verdict::Ok,
            1.0,
            Source::Server,
            "",
        )]);
        let mut all = report.all_signals();
        assert_eq!(
            all.iter()
                .map(|signal| signal.id.as_str())
                .collect::<Vec<_>>(),
            ["l1.navigator.webdriver", "l5.rit.ok"]
        );
        all.clear();
        assert_eq!(report.client.signals.as_ref().unwrap().len(), 1);
    }
}
