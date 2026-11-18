use crate::audit::record::Record;
use std::io;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub(super) fn stamp(record: &mut Record) -> io::Result<()> {
    let now = OffsetDateTime::now_utc();
    if record.event_id.is_empty() {
        let millis = now.unix_timestamp_nanos() / 1_000_000;
        if !(0..(1_i128 << 48)).contains(&millis) {
            return Err(io::Error::other("audit clock outside UUIDv7 range"));
        }
        let mut bytes = [0; 16];
        bytes[..6].copy_from_slice(&(millis as u64).to_be_bytes()[2..]);
        getrandom::fill(&mut bytes[6..])
            .map_err(|_| io::Error::other("audit event entropy unavailable"))?;
        bytes[6] = (bytes[6] & 15) | 0x70;
        bytes[8] = (bytes[8] & 63) | 0x80;
        let h = hex::encode(bytes);
        record.event_id = format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        );
    }
    if record.ts.is_empty() {
        record.ts = now.format(&Rfc3339).map_err(io::Error::other)?;
    }
    if record.event_version.is_empty() {
        record.event_version = crate::audit::record::events::EVENT_SCHEMA_VERSION.into();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Legacy id_test.go TestNewEventIDUnique: ten thousand mints, most of
    /// them sharing a millisecond prefix, never collide.
    #[test]
    fn event_ids_are_unique_across_many_mints() {
        let mut seen = HashSet::new();
        for _ in 0..10_000 {
            let mut record = Record::default();
            stamp(&mut record).unwrap();
            assert!(seen.insert(record.event_id), "duplicate event id");
        }
    }

    #[test]
    fn caller_set_fields_are_kept_and_minted_ts_is_rfc3339() {
        let mut record = Record {
            event_id: "caller-id".into(),
            ts: "2026-01-02T03:04:05Z".into(),
            event_version: "2.0.0".into(),
            ..Record::default()
        };
        stamp(&mut record).unwrap();
        assert_eq!(
            (
                record.event_id.as_str(),
                record.ts.as_str(),
                record.event_version.as_str()
            ),
            ("caller-id", "2026-01-02T03:04:05Z", "2.0.0")
        );
        let mut minted = Record::default();
        stamp(&mut minted).unwrap();
        assert!(OffsetDateTime::parse(&minted.ts, &Rfc3339).is_ok());
    }
}
