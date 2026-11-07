//! HTTP/2 observations retain settings and header order before normalization.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod monitor;
pub use monitor::FrameMonitor;

/// One SETTINGS pair in wire order, including duplicate identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    /// Unsigned setting identifier.
    pub id: u16,
    /// Unsigned setting value.
    pub value: u32,
}

/// Captured HTTP/2 connection characteristics. This structure does not assert
/// that a complete connection preface or header block has been observed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fingerprint {
    /// SETTINGS pairs in wire order.
    pub settings: Vec<Setting>,
    /// First connection WINDOW_UPDATE increment.
    pub window_update: u32,
    /// Separate PRIORITY tuples streamID:excl:dep:weight.
    pub priorities: Vec<String>,
    /// HEADERS frame's excl:dep:weight tuple, if present.
    pub headers_priority: String,
    /// Pseudo-header order as m,a,s,p abbreviations.
    pub pseudo_order: Vec<String>,
    /// Regular header names in wire order.
    pub header_order: Vec<String>,
}

impl Fingerprint {
    /// Render the canonical Akamai pipe-separated observation. HEADERS priority
    /// is intentionally distinct from the separate PRIORITY-frame section.
    pub fn akamai(&self) -> String {
        let settings = self
            .settings
            .iter()
            .map(|s| format!("{}:{}", s.id, s.value))
            .collect::<Vec<_>>()
            .join(";");
        let priorities = if self.priorities.is_empty() {
            "0".into()
        } else {
            self.priorities.join(",")
        };
        format!(
            "{settings}|{}|{priorities}|{}",
            self.window_update,
            self.pseudo_string()
        )
    }

    /// Join observed pseudo-header order without sorting.
    pub fn pseudo_string(&self) -> String {
        self.pseudo_order.join(",")
    }

    /// Resolve the frozen structural engine heuristic. Browser pseudo-order
    /// takes precedence over library SETTINGS profiles.
    pub fn engine(&self) -> &'static str {
        match self.pseudo_string().as_str() {
            "m,a,s,p" => "chrome",
            "m,p,a,s" => "firefox",
            "m,s,p,a" => "safari",
            _ => {
                let settings = self.settings_map();
                if settings.contains_key(&3) && settings.get(&4) == Some(&65535) {
                    "go"
                } else {
                    "unknown"
                }
            }
        }
    }

    /// Last duplicate SETTINGS value wins in the comparison view; wire order
    /// remains available in settings for fingerprints and anomaly observations.
    pub fn settings_map(&self) -> BTreeMap<u16, u32> {
        self.settings
            .iter()
            .map(|setting| (setting.id, setting.value))
            .collect()
    }

    /// Test presence without assuming unique SETTINGS identifiers.
    pub fn has_setting(&self, id: u16) -> bool {
        self.settings.iter().any(|setting| setting.id == id)
    }

    /// Sorted IDs retain duplicates for stable profile comparisons.
    pub fn sorted_setting_ids(&self) -> Vec<u16> {
        let mut ids = self
            .settings
            .iter()
            .map(|setting| setting.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_order_and_duplicate_settings_are_not_normalized_away() {
        let mut fingerprint = Fingerprint {
            settings: vec![
                Setting { id: 4, value: 1 },
                Setting { id: 3, value: 100 },
                Setting {
                    id: 4,
                    value: 65535,
                },
            ],
            headers_priority: "0:0:15".into(),
            ..Default::default()
        };
        assert_eq!(fingerprint.engine(), "go");
        assert_eq!(fingerprint.akamai(), "4:1;3:100;4:65535|0|0|");
        assert_eq!(fingerprint.sorted_setting_ids(), [3, 4, 4]);
        assert!(fingerprint.has_setting(3));
        fingerprint.pseudo_order = ["m", "a", "s", "p"].map(str::to_owned).into();
        assert_eq!(fingerprint.engine(), "chrome");
    }
}
