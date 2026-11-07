use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{Date, Month, OffsetDateTime, format_description::well_known::Rfc3339};

/// RFC3339 timestamp with the Go zero-time default, not Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub OffsetDateTime);

impl Default for Timestamp {
    fn default() -> Self {
        Self(
            Date::from_calendar_date(1, Month::January, 1)
                .expect("valid zero date")
                .midnight()
                .assume_utc(),
        )
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0
            .format(&Rfc3339)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        OffsetDateTime::parse(&text, &Rfc3339)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_time_and_nanosecond_wire_formats() {
        assert_eq!(
            serde_json::to_string(&Timestamp::default()).unwrap(),
            "\"0001-01-01T00:00:00Z\""
        );
        for value in ["2026-11-07T04:00:00.000009+09:00", "2026-11-06T19:00:00Z"] {
            let encoded = format!("\"{value}\"");
            let parsed: Timestamp = serde_json::from_str(&encoded).unwrap();
            assert_eq!(serde_json::to_string(&parsed).unwrap(), encoded);
        }
        assert!(serde_json::from_str::<Timestamp>("\"not-a-time\"").is_err());
    }
}
