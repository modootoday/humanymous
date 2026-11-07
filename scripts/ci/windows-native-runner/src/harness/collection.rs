//! Pure failure collection: completed child evidence may fail without starving peers.
use serde_json::{Value, json};
use std::io;
pub(super) struct Ledger {
    required: usize,
    completed: usize,
    fatal: bool,
    failures: Vec<Value>,
}
impl Ledger {
    pub fn new(required: usize) -> Self {
        Self {
            required,
            completed: 0,
            fatal: false,
            failures: Vec::new(),
        }
    }
    pub fn completed(&mut self, index: usize, target: &Value, failure: Option<&str>) -> bool {
        if self.fatal {
            return false;
        }
        self.completed += 1;
        if let Some(code) = failure {
            self.failures
                .push(json!({"index":index,"target":target,"code":code}));
        }
        true
    }
    pub fn missing_native(&mut self) {
        self.failures
            .push(json!({"index":null,"target":"native_process","code":"required-native-harness"}));
    }
    pub fn abort(&mut self) {
        self.fatal = true;
    }
    pub fn snapshot(&self, progress: &mut Value) {
        let execution_complete =
            !self.fatal && self.required > 0 && self.completed == self.required;
        progress["executed_harnesses"] = json!(self.completed);
        progress["execution_complete"] = json!(execution_complete);
        progress["fatal_stop"] = json!(self.fatal);
        progress["failures"] = json!(self.failures);
        progress["complete"] = json!(execution_complete && self.failures.is_empty());
    }
    pub fn finish(&self, progress: &mut Value) -> io::Result<()> {
        self.snapshot(progress);
        if progress["complete"] != true {
            return Err(io::Error::other(
                "required harness coverage incomplete; inspect indexed failures",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn early_failed_coverage_preserves_later_completed_receipts_and_final_failure() {
        let mut ledger = Ledger::new(2);
        assert!(ledger.completed(0, &json!("host_process"), Some("coverage-accounting")));
        assert!(ledger.completed(1, &json!("native_process"), None));
        let mut progress = json!({});
        assert!(ledger.finish(&mut progress).is_err());
        assert_eq!(progress["executed_harnesses"], 2);
        assert_eq!(progress["execution_complete"], true);
        assert_eq!(progress["complete"], false);
        assert_eq!(progress["failures"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn fatal_security_or_transport_stop_refuses_later_collection() {
        let mut ledger = Ledger::new(2);
        ledger.completed(0, &json!("completed"), None);
        ledger.abort();
        assert!(!ledger.completed(1, &json!("unsafe"), None));
        let mut progress = json!({});
        assert!(ledger.finish(&mut progress).is_err());
        assert_eq!(progress["executed_harnesses"], 1);
        assert_eq!(progress["fatal_stop"], true);
        assert_eq!(progress["execution_complete"], false);
    }
}
