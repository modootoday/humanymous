use serde::Serialize;
use std::collections::BTreeMap;

/// Observer state errors. No failure changes the last accepted receipt time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    /// Missing node identity, nonpositive cadence or unsafe interval arithmetic.
    InvalidRegistration,
    /// Registration must not silently reset an existing node's deadline.
    AlreadyRegistered,
    /// Unknown nodes cannot add themselves by sending receipts.
    UnknownNode,
    /// Observer clock regressed; do not renew or weaken the previous deadline.
    ClockRegressed,
}
struct Node {
    interval_ns: i128,
    last_ns: i128,
}

/// Public status without raw keys or receipt payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeStatus {
    /// Registered identity.
    pub node: String,
    /// Signed observer age in milliseconds, matching historical display units.
    #[serde(rename = "lastSeenMs")]
    pub last_seen_ms: i128,
    /// Strictly more than twice the configured cadence without a receipt.
    pub missing: bool,
}

/// Independent observer's expected-node set. Caller authenticates node receipts
/// and rejects network replay before record_receipt; this is not a signed
/// heartbeat protocol. Running this only inside the watched process cannot
/// detect that process's suppression. Persist and monitor externally to deploy.
#[derive(Default)]
pub struct NodeRegistry {
    nodes: BTreeMap<String, Node>,
}
impl NodeRegistry {
    /// Register a trusted expected node and observer-clock start in nanoseconds.
    /// Re-registration is refused so it cannot conceal a missing heartbeat.
    pub fn expect(
        &mut self,
        node_id: String,
        interval_ns: i128,
        now_ns: i128,
    ) -> Result<(), RegistryError> {
        if node_id.is_empty() || interval_ns <= 0 || interval_ns.checked_mul(2).is_none() {
            return Err(RegistryError::InvalidRegistration);
        }
        if self.nodes.contains_key(&node_id) {
            return Err(RegistryError::AlreadyRegistered);
        }
        self.nodes.insert(
            node_id,
            Node {
                interval_ns,
                last_ns: now_ns,
            },
        );
        Ok(())
    }
    /// Record an already authenticated fresh receipt using the observer's
    /// clock, never a remote claimed timestamp. No network authentication or
    /// replay check is implied by this pure state transition.
    pub fn record_receipt(&mut self, node_id: &str, now_ns: i128) -> Result<(), RegistryError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(RegistryError::UnknownNode)?;
        if now_ns < node.last_ns {
            return Err(RegistryError::ClockRegressed);
        }
        node.last_ns = now_ns;
        Ok(())
    }
    /// Deterministic node snapshots. Clock regression/overflow is explicit and
    /// must not produce a reassuring missing=false result.
    pub fn nodes(&self, now_ns: i128) -> Result<Vec<NodeStatus>, RegistryError> {
        self.nodes
            .iter()
            .map(|(id, node)| {
                let age = now_ns
                    .checked_sub(node.last_ns)
                    .filter(|age| *age >= 0)
                    .ok_or(RegistryError::ClockRegressed)?;
                Ok(NodeStatus {
                    node: id.clone(),
                    last_seen_ms: age / 1_000_000,
                    missing: age > node.interval_ns * 2,
                })
            })
            .collect()
    }
    /// Sorted identities whose two-cadence grace period has expired.
    pub fn missing(&self, now_ns: i128) -> Result<Vec<String>, RegistryError> {
        Ok(self
            .nodes(now_ns)?
            .into_iter()
            .filter(|node| node.missing)
            .map(|node| node.node)
            .collect())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suppression_resume_and_exact_boundary_match_source_regression() {
        let sec = 1_000_000_000;
        let mut registry = NodeRegistry::default();
        for node in ["proxy-1", "proxy-2"] {
            registry.expect(node.into(), sec, 0).unwrap();
            registry.record_receipt(node, sec).unwrap();
        }
        assert!(registry.missing(sec).unwrap().is_empty());
        assert!(registry.missing(3 * sec).unwrap().is_empty());
        registry.record_receipt("proxy-1", 3 * sec).unwrap();
        assert_eq!(
            registry.missing(3 * sec + sec / 2).unwrap(),
            vec!["proxy-2"]
        );
        registry.record_receipt("proxy-2", 4 * sec).unwrap();
        assert!(registry.missing(4 * sec + sec / 2).unwrap().is_empty());
        assert_eq!(
            registry.nodes(4 * sec + sec / 2).unwrap()[0].last_seen_ms,
            1500
        );
    }
    #[test]
    fn invalid_and_regressing_inputs_cannot_refresh_liveness() {
        let mut registry = NodeRegistry::default();
        assert_eq!(
            registry.expect("".into(), 1, 0),
            Err(RegistryError::InvalidRegistration)
        );
        assert_eq!(
            registry.expect("n".into(), i128::MAX, 0),
            Err(RegistryError::InvalidRegistration)
        );
        registry.expect("n".into(), 10, 100).unwrap();
        assert_eq!(
            registry.expect("n".into(), 10, 1000),
            Err(RegistryError::AlreadyRegistered)
        );
        assert_eq!(
            registry.record_receipt("n", 99),
            Err(RegistryError::ClockRegressed)
        );
        assert_eq!(
            registry.record_receipt("unknown", 100),
            Err(RegistryError::UnknownNode)
        );
        assert_eq!(registry.nodes(99), Err(RegistryError::ClockRegressed));
        assert_eq!(registry.missing(121).unwrap(), vec!["n"]);
    }
}
