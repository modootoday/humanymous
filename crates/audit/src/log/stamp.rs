use crate::record::Record;
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
        record.event_version = "1.0.0".into();
    }
    Ok(())
}
