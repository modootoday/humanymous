//! One field emitter for compatibility bytes and bounded hashing.
use super::{Record, confidence};
use crate::record::{normalization::nfc_chars, ordering};
use std::io::{self, Write};
#[cfg(test)]
mod tests;

pub(super) struct Limits {
    pub bytes: usize,
    pub items: usize,
}
impl Limits {
    pub fn compatibility() -> Self {
        Self {
            bytes: usize::MAX,
            items: usize::MAX,
        }
    }
}
fn budget() -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        "canonical resource limit exceeded",
    )
}
struct Limited<'a, W: Write> {
    inner: &'a mut W,
    written: usize,
    limit: usize,
}
impl<W: Write> Limited<'_, W> {
    fn remaining(&self) -> usize {
        self.limit - self.written
    }
}
impl<W: Write> Write for Limited<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining() {
            return Err(budget());
        }
        let count = self.inner.write(bytes)?;
        self.written = self.written.checked_add(count).ok_or_else(budget)?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
struct Count {
    bytes: usize,
    limit: usize,
}
fn disagreement() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "canonical count and replay disagree",
    )
}
struct Replay<'a> {
    inner: &'a mut dyn Write,
    bytes: usize,
    expected: usize,
}
impl Write for Replay<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.expected - self.bytes {
            return Err(disagreement());
        }
        let count = self.inner.write(bytes)?;
        self.bytes = self.bytes.checked_add(count).ok_or_else(disagreement)?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self.bytes.checked_add(bytes.len()).ok_or_else(budget)?;
        if next > self.limit {
            return Err(budget());
        }
        self.bytes = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn field<W: Write>(
    out: &mut Limited<'_, W>,
    key: &str,
    emit: impl Fn(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    let key_len = u32::try_from(key.len()).map_err(|_| budget())?;
    let header = key.len().checked_add(8).ok_or_else(budget)?;
    let available = out.remaining().checked_sub(header).ok_or_else(budget)?;
    let mut count = Count {
        bytes: 0,
        limit: available.min(u32::MAX as usize),
    };
    emit(&mut count)?;
    let length = u32::try_from(count.bytes).map_err(|_| budget())?;
    out.write_all(&key_len.to_be_bytes())?;
    out.write_all(key.as_bytes())?;
    out.write_all(&length.to_be_bytes())?;
    let mut replay = Replay {
        inner: out,
        bytes: 0,
        expected: count.bytes,
    };
    emit(&mut replay)?;
    if replay.bytes != count.bytes {
        return Err(disagreement());
    }
    Ok(())
}
fn normalized(out: &mut dyn Write, value: &str) -> io::Result<()> {
    let mut bytes = [0; 4];
    for character in nfc_chars(value) {
        out.write_all(character.encode_utf8(&mut bytes).as_bytes())?;
    }
    Ok(())
}
fn scalar<W: Write>(out: &mut Limited<'_, W>, key: &str, value: &str) -> io::Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    field(out, key, |sink| normalized(sink, value))
}
fn raw<W: Write>(out: &mut Limited<'_, W>, key: &str, value: &str) -> io::Result<()> {
    field(out, key, |sink| sink.write_all(value.as_bytes()))
}

pub(super) fn write_record(
    record: &Record,
    sink: &mut impl Write,
    limits: Limits,
) -> io::Result<()> {
    let rules = record.triggered_rules.as_deref().unwrap_or_default();
    let signals = record.top_signals.as_deref().unwrap_or_default();
    if rules.len() > limits.items || signals.len() > limits.items {
        return Err(budget());
    }
    let mut rules = rules.iter().collect::<Vec<_>>();
    rules.sort();
    let mut signals = signals.iter().collect::<Vec<_>>();
    ordering::sort_signals(&mut signals);
    let mut out = Limited {
        inner: sink,
        written: 0,
        limit: limits.bytes,
    };
    scalar(&mut out, "event_id", &record.event_id)?;
    raw(&mut out, "seq", &(record.seq as i64).to_string())?;
    for (key, value) in [
        ("node_id", &record.node_id),
        ("ts", &record.ts),
        ("event_type", &record.event_type),
        ("event_version", &record.event_version),
        ("actor.kind", &record.actor.kind),
        ("actor.id", &record.actor.id_pseudonym),
        ("tenant_id", &record.tenant_id),
        ("session_pseudonym", &record.session_pseudonym),
        ("request_id", &record.request_id),
        ("correlation_id", &record.correlation_id),
        ("host", &record.host),
        ("route_class", &record.route_class),
        ("verdict", &record.verdict),
    ] {
        scalar(&mut out, key, value)?;
    }
    raw(&mut out, "risk_score", &record.risk_score.to_string())?;
    // Rules/verdicts retain raw bytes; normalization applies to signal IDs only.
    field(&mut out, "triggered_rules", |sink| {
        sink.write_all(b"[")?;
        for (index, rule) in rules.iter().enumerate() {
            if index != 0 {
                sink.write_all(b",")?;
            }
            sink.write_all(rule.as_bytes())?;
        }
        sink.write_all(b"]")
    })?;
    field(&mut out, "top_signals", |sink| {
        sink.write_all(b"[")?;
        for (index, signal) in signals.iter().enumerate() {
            if index != 0 {
                sink.write_all(b",")?;
            }
            normalized(sink, &signal.id)?;
            sink.write_all(b"|")?;
            sink.write_all(signal.verdict.as_bytes())?;
            sink.write_all(b"|")?;
            sink.write_all(confidence(signal.conf).as_bytes())?;
        }
        sink.write_all(b"]")
    })?;
    for (key, value) in [
        ("enforcement_action", &record.enforcement_action),
        ("enforcement_mode", &record.enforcement_mode),
        ("fail_mode", &record.fail_mode),
        ("fail_reason", &record.fail_reason),
    ] {
        scalar(&mut out, key, value)?;
    }
    raw(&mut out, "latency_us", &record.latency_us.to_string())?;
    if let Some(upstream) = &record.upstream {
        raw(&mut out, "upstream.status", &upstream.status.to_string())?;
        scalar(&mut out, "upstream.error_class", &upstream.error_class)?;
    }
    if let Some(tls) = &record.tls {
        for (key, value) in [
            ("tls.ja4", &tls.ja4_pseudonym),
            ("tls.h2fp", &tls.h2fp_pseudonym),
            ("tls.sni", &tls.sni_pseudonym),
        ] {
            scalar(&mut out, key, value)?;
        }
    }
    for (key, value) in [
        ("config_version", &record.config_version),
        ("key_id", &record.key_id),
        ("data_class", &record.data_class),
    ] {
        scalar(&mut out, key, value)?;
    }
    Ok(())
}
