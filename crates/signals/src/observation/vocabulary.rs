use serde::{Deserialize, Serialize};

macro_rules! wire_enum {
    ($name:ident, $description:literal, {$($variant:ident => $value:literal),+ $(,)?}) => {
        #[doc = $description]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(from = "String", into = "String")]
        pub enum $name {
            $(#[doc = $value] $variant,)+
            /// An unrecognized wire value, preserved as in the Go string type.
            Other(String),
        }
        impl $name {
            /// Exact wire representation.
            pub fn as_str(&self) -> &str {
                match self { $(Self::$variant => $value,)+ Self::Other(value) => value }
            }
        }
        impl Default for $name {
            fn default() -> Self { Self::Other(String::new()) }
        }
        impl From<String> for $name {
            fn from(value: String) -> Self {
                match value.as_str() { $($value => Self::$variant,)+ _ => Self::Other(value) }
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self { value.as_str().to_owned() }
        }
    };
}

wire_enum!(Layer, "Detection-layer identity.", {
    Static => "L1", Fingerprint => "L2", Integrity => "L3", Behavioral => "L4",
    Network => "L5", Consistency => "L6", Scoring => "L7",
});
wire_enum!(Verdict, "Per-signal qualitative judgment; distinct from session policy.", {
    Ok => "OK", Suspicious => "SUSPICIOUS", Bot => "BOT", Unknown => "UNKNOWN",
});
wire_enum!(Source, "Observation collection plane.", {
    Js => "js", Wasm => "wasm", Server => "server",
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_values_and_unknowns_round_trip() {
        for value in ["OK", "SUSPICIOUS", "BOT", "UNKNOWN", "future", ""] {
            let verdict = Verdict::from(value.to_owned());
            let json = serde_json::to_string(&verdict).unwrap();
            assert_eq!(
                serde_json::from_str::<Verdict>(&json).unwrap().as_str(),
                value
            );
        }
        assert_eq!(serde_json::to_string(&Layer::Network).unwrap(), "\"L5\"");
        assert_eq!(serde_json::to_string(&Source::Wasm).unwrap(), "\"wasm\"");
    }
}
