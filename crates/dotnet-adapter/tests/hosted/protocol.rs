//! Pure prebuilt-host framing and bounded queue. No IO, spawn or cleanup proof.
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

const FRAME_LIMIT: usize = 4096;
const STREAM_LIMIT: usize = 256 * 1024;
const QUEUE_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum HandoffMode {
    Normal,
    Rewrite,
    ResponseStarted,
    Expire,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Control {
    GcStart,
    GcStop,
    DisposeHost,
    HandoffMode { mode: HandoffMode },
    Stop,
    Counters,
}

struct Controls {
    send: SyncSender<Vec<u8>>,
    failed: bool,
    stopped: bool,
}
impl Controls {
    fn new() -> (Self, Receiver<Vec<u8>>) {
        let (send, receive) = sync_channel(QUEUE_LIMIT);
        (
            Self {
                send,
                failed: false,
                stopped: false,
            },
            receive,
        )
    }
    fn offer(&mut self, control: Control) -> Result<(), &'static str> {
        if self.failed || self.stopped {
            return Err("control queue already terminal");
        }
        let mut bytes = serde_json::to_vec(&control).map_err(|_| "control encoding")?;
        bytes.push(b'\n');
        if bytes.len() > 1024 || self.send.try_send(bytes).is_err() {
            self.failed = true;
            return Err("control queue unavailable");
        }
        self.stopped = matches!(control, Control::Stop);
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(super) enum Frame {
    #[serde(rename = "starting")]
    Starting { pid: u32 },
    #[serde(rename = "ready")]
    Ready {
        pid: u32,
        address: String,
        runtime: String,
        dynamic_code: bool,
    },
    #[serde(rename = "application-contact")]
    Contact {
        contacts: u64,
        mutations: u64,
        body_length: u64,
    },
    #[serde(rename = "counters")]
    Counters {
        contacts: u64,
        mutations: u64,
        collections: u64,
        #[serde(deserialize_with = "resources")]
        native_resources: Option<u64>,
    },
    #[serde(rename = "stopped")]
    Stopped {
        contacts: u64,
        mutations: u64,
        collections: u64,
    },
}

fn resources<'de, D: Deserializer<'de>>(input: D) -> Result<Option<u64>, D::Error> {
    Option::<u64>::deserialize(input)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Starting,
    Ready,
    Running,
    Stopped,
    Eof,
    Failed,
}

pub(super) struct Ingress {
    send: SyncSender<Frame>,
    pid: u32,
    phase: Phase,
    bytes: usize,
    pending: Vec<u8>,
}

impl Ingress {
    pub(super) fn new(pid: u32) -> (Self, Receiver<Frame>) {
        let (send, receive) = sync_channel(QUEUE_LIMIT);
        (
            Self {
                send,
                pid,
                phase: Phase::Starting,
                bytes: 0,
                pending: Vec::new(),
            },
            receive,
        )
    }

    fn offer(&mut self, line: &[u8]) -> Result<(), &'static str> {
        let result = self.offer_inner(line);
        if result.is_err() {
            self.phase = Phase::Failed;
        }
        result
    }

    // Native output chunks have no line boundaries. Retain at most one bounded
    // partial frame; never infer stream completion from a temporarily empty queue.
    pub(super) fn offer_chunk(&mut self, chunk: &[u8]) -> Result<(), &'static str> {
        let result = self.offer_chunk_inner(chunk);
        if result.is_err() {
            self.phase = Phase::Failed;
        }
        result
    }

    fn offer_chunk_inner(&mut self, chunk: &[u8]) -> Result<(), &'static str> {
        if matches!(self.phase, Phase::Failed | Phase::Stopped | Phase::Eof) {
            return Err("host stream already terminal");
        }
        let total = self
            .bytes
            .checked_add(self.pending.len())
            .and_then(|count| count.checked_add(chunk.len()))
            .ok_or("stream overflow")?;
        if total > STREAM_LIMIT {
            return Err("host stream byte budget");
        }
        for &byte in chunk {
            if matches!(self.phase, Phase::Stopped | Phase::Eof) {
                return Err("host bytes after stop");
            }
            if self.pending.len() == FRAME_LIMIT {
                return Err("host frame bounds");
            }
            self.pending.push(byte);
            if byte == b'\n' {
                let line = std::mem::take(&mut self.pending);
                self.offer_inner(&line)?;
            }
        }
        Ok(())
    }

    fn offer_inner(&mut self, line: &[u8]) -> Result<(), &'static str> {
        if matches!(self.phase, Phase::Failed | Phase::Stopped | Phase::Eof) {
            return Err("host stream already terminal");
        }
        if line.is_empty() || line.len() > FRAME_LIMIT || line.last() != Some(&b'\n') {
            return Err("host frame bounds");
        }
        self.bytes = self
            .bytes
            .checked_add(line.len())
            .ok_or("stream overflow")?;
        if self.bytes > STREAM_LIMIT {
            return Err("host stream byte budget");
        }
        let frame: Frame = serde_json::from_slice(line).map_err(|_| "host frame schema")?;
        self.phase = match (&frame, self.phase) {
            (Frame::Starting { pid }, Phase::Starting) if *pid == self.pid => Phase::Ready,
            (Frame::Ready { pid, .. }, Phase::Ready) if *pid == self.pid => Phase::Running,
            (Frame::Contact { .. } | Frame::Counters { .. }, Phase::Running) => Phase::Running,
            (Frame::Stopped { .. }, Phase::Running) => Phase::Stopped,
            _ => return Err("host frame sequence or PID"),
        };
        // Never block an IO worker when the controller has stopped consuming.
        self.send
            .try_send(frame)
            .map_err(|_| "host frame queue unavailable")
    }

    pub(super) fn eof(&mut self) -> Result<(), &'static str> {
        if self.phase != Phase::Stopped || !self.pending.is_empty() {
            self.phase = Phase::Failed;
            return Err("host EOF before successful protocol stop");
        }
        self.phase = Phase::Eof;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const START: &[u8] = b"{\"kind\":\"starting\",\"pid\":7}\n";
    const READY: &[u8] = b"{\"kind\":\"ready\",\"pid\":7,\"address\":\"http://127.0.0.1:1\",\"runtime\":\".NET 10.0.14\",\"dynamic_code\":true}\n";
    const COUNTERS: &[u8] = b"{\"kind\":\"counters\",\"contacts\":0,\"mutations\":0,\"collections\":1,\"native_resources\":0}\n";
    const CONTACT: &[u8] =
        b"{\"kind\":\"application-contact\",\"contacts\":1,\"mutations\":0,\"body_length\":6}\n";
    const STOP: &[u8] =
        b"{\"kind\":\"stopped\",\"contacts\":0,\"mutations\":0,\"collections\":1}\n";

    #[test]
    fn byte_chunks_preserve_split_and_coalesced_frames() {
        let (mut ingress, receive) = Ingress::new(7);
        for byte in START {
            ingress.offer_chunk(&[*byte]).unwrap();
        }
        let rest = [READY, COUNTERS, STOP].concat();
        ingress.offer_chunk(&rest).unwrap();
        assert!(ingress.pending.is_empty());
        ingress.eof().unwrap();
        assert_eq!(receive.try_iter().count(), 4);
    }

    #[test]
    fn partial_frame_eof_and_oversize_are_sticky_failures() {
        let (mut partial, _receive) = Ingress::new(7);
        partial.offer_chunk(&START[..START.len() - 1]).unwrap();
        assert!(partial.eof().is_err());
        assert!(partial.offer_chunk(b"\n").is_err());
        let (mut oversized, _receive) = Ingress::new(7);
        oversized.offer_chunk(&vec![b'x'; FRAME_LIMIT]).unwrap();
        assert!(oversized.offer_chunk(b"\n").is_err());
        assert_eq!(oversized.pending.len(), FRAME_LIMIT);
        assert!(oversized.offer_chunk(START).is_err());
    }

    #[test]
    fn chunks_count_pending_bytes_and_refuse_trailing_stop_data() {
        let (mut capped, _receive) = Ingress::new(7);
        capped.bytes = STREAM_LIMIT - 2;
        capped.offer_chunk(b"{").unwrap();
        assert!(capped.offer_chunk(b"xx").is_err());
        let (mut trailing, _receive) = Ingress::new(7);
        assert!(
            trailing
                .offer_chunk(&[START, READY, STOP, b"x"].concat())
                .is_err()
        );
        assert!(trailing.eof().is_err());
    }

    #[test]
    fn exact_pid_sequence_requires_stop_then_eof() {
        let (mut ingress, receive) = Ingress::new(7);
        for line in [START, READY, COUNTERS, CONTACT, STOP] {
            ingress.offer(line).unwrap();
        }
        ingress.eof().unwrap();
        let frames: Vec<_> = receive.try_iter().collect();
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[0], Frame::Starting { pid: 7 });
        assert_eq!(
            frames[1],
            Frame::Ready {
                pid: 7,
                address: "http://127.0.0.1:1".into(),
                runtime: ".NET 10.0.14".into(),
                dynamic_code: true
            }
        );
        assert_eq!(
            frames[2],
            Frame::Counters {
                contacts: 0,
                mutations: 0,
                collections: 1,
                native_resources: Some(0)
            }
        );
        assert_eq!(
            frames[3],
            Frame::Contact {
                contacts: 1,
                mutations: 0,
                body_length: 6
            }
        );
        assert_eq!(
            frames[4],
            Frame::Stopped {
                contacts: 0,
                mutations: 0,
                collections: 1
            }
        );
        assert_eq!(ingress.phase, Phase::Eof);
        assert!(ingress.offer(COUNTERS).is_err());
    }

    #[test]
    fn malformed_duplicate_unknown_and_wrong_pid_are_sticky_failures() {
        for bad in [
            b"{\"kind\":\"starting\",\"pid\":8}\n".as_slice(),
            b"{\"kind\":\"starting\",\"pid\":7,\"pid\":7}\n",
            b"{\"kind\":\"starting\",\"pid\":7,\"extra\":0}\n",
            b"{\"kind\":\"generic-native-abi\",\"passed\":1}\n",
            b"{\"kind\":\"starting\",\"pid\":7}",
            READY,
        ] {
            let (mut ingress, _receive) = Ingress::new(7);
            assert!(ingress.offer(bad).is_err());
            assert!(ingress.offer(START).is_err());
            assert_eq!(ingress.phase, Phase::Failed);
        }
    }

    #[test]
    fn full_or_disconnected_queue_fails_without_blocking() {
        let (mut ingress, receive) = Ingress::new(7);
        ingress.offer(START).unwrap();
        ingress.offer(READY).unwrap();
        for _ in 2..QUEUE_LIMIT {
            ingress.offer(COUNTERS).unwrap();
        }
        assert!(ingress.offer(COUNTERS).is_err());
        assert_eq!(receive.try_iter().count(), QUEUE_LIMIT);
        assert_eq!(ingress.phase, Phase::Failed);
        let (mut disconnected, receive) = Ingress::new(7);
        drop(receive);
        assert!(disconnected.offer(START).is_err());
    }

    #[test]
    fn draining_queue_does_not_reset_total_output_budget() {
        let (mut ingress, receive) = Ingress::new(7);
        ingress.offer(START).unwrap();
        ingress.offer(READY).unwrap();
        while ingress.offer(COUNTERS).is_ok() {
            receive.try_iter().for_each(drop);
        }
        assert!(ingress.bytes > STREAM_LIMIT);
        assert_eq!(ingress.phase, Phase::Failed);
    }

    #[test]
    fn premature_eof_and_oversized_frames_cannot_settle_protocol() {
        let (mut ingress, _receive) = Ingress::new(7);
        ingress.offer(START).unwrap();
        assert!(ingress.eof().is_err());
        let (mut oversized, _receive) = Ingress::new(7);
        let mut line = vec![b' '; FRAME_LIMIT + 1];
        *line.last_mut().unwrap() = b'\n';
        assert!(oversized.offer(&line).is_err());
    }

    #[test]
    fn native_metrics_must_be_explicit_even_when_null() {
        let (mut ingress, _receive) = Ingress::new(7);
        ingress.offer(START).unwrap();
        ingress.offer(READY).unwrap();
        assert!(
            ingress
                .offer(
                    b"{\"kind\":\"counters\",\"contacts\":0,\"mutations\":0,\"collections\":0}\n"
                )
                .is_err()
        );
        let (mut valid, receive) = Ingress::new(7);
        valid.offer(START).unwrap();
        valid.offer(READY).unwrap();
        valid.offer(b"{\"kind\":\"counters\",\"contacts\":0,\"mutations\":0,\"collections\":0,\"native_resources\":null}\n").unwrap();
        assert!(matches!(
            receive.try_iter().last(),
            Some(Frame::Counters {
                native_resources: None,
                ..
            })
        ));
    }

    #[test]
    fn controls_have_closed_spellings_and_stop_is_terminal() {
        let (mut controls, receive) = Controls::new();
        for control in [
            Control::GcStart,
            Control::GcStop,
            Control::DisposeHost,
            Control::HandoffMode {
                mode: HandoffMode::Normal,
            },
            Control::HandoffMode {
                mode: HandoffMode::Rewrite,
            },
            Control::HandoffMode {
                mode: HandoffMode::ResponseStarted,
            },
            Control::HandoffMode {
                mode: HandoffMode::Expire,
            },
            Control::Counters,
            Control::Stop,
        ] {
            controls.offer(control).unwrap();
        }
        let encoded: Vec<_> = receive
            .try_iter()
            .map(|line| {
                assert!(line.len() <= 1024 && line.last() == Some(&b'\n'));
                serde_json::from_slice::<serde_json::Value>(&line).unwrap()
            })
            .collect();
        assert_eq!(encoded.len(), 9);
        assert_eq!(encoded[0]["operation"], "gc-start");
        assert_eq!(encoded[5]["mode"], "response-started");
        assert_eq!(encoded[8]["operation"], "stop");
        assert!(controls.offer(Control::Counters).is_err());
    }

    #[test]
    fn control_backpressure_and_disconnection_never_block_or_recover() {
        let (mut controls, receive) = Controls::new();
        for _ in 0..QUEUE_LIMIT {
            controls.offer(Control::Counters).unwrap();
        }
        assert!(controls.offer(Control::Counters).is_err());
        assert_eq!(receive.try_iter().count(), QUEUE_LIMIT);
        assert!(controls.offer(Control::Stop).is_err());
        let (mut disconnected, receive) = Controls::new();
        drop(receive);
        assert!(disconnected.offer(Control::Counters).is_err());
        assert!(disconnected.offer(Control::Stop).is_err());
    }
}
