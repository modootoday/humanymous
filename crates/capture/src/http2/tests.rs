use super::*;

fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut result = vec![(len >> 16) as u8, (len >> 8) as u8, len as u8, kind, flags];
    result.extend(stream.to_be_bytes());
    result.extend(payload);
    result
}
fn prefix() -> Vec<u8> {
    [PREFACE, &frame(4, 0, 0, &[])].concat()
}
fn headers() -> Vec<u8> {
    // :method GET, :authority example.test, :scheme https, :path /.
    let mut bytes = vec![0x82, 0x01, 12];
    bytes.extend(b"example.test");
    bytes.extend([0x87, 0x84]);
    bytes
}
fn literal(name: &[u8], value: &[u8]) -> Vec<u8> {
    let mut result = vec![0, name.len() as u8];
    result.extend(name);
    result.push(value.len() as u8);
    result.extend(value);
    result
}
#[test]
fn every_split_retains_settings_duplicates_and_complete_header_order() {
    let settings = [0, 4, 0, 0, 0, 1, 0, 3, 0, 0, 0, 100, 0, 4, 0, 0, 0xff, 0xff];
    let mut block = headers();
    block.extend(literal(b"x-tag", b"one"));
    block.extend(literal(b"x-tag", b"two"));
    for split in 0..=block.len() {
        let wire = [
            PREFACE.to_vec(),
            frame(4, 0, 0, &settings),
            frame(8, 0, 0, &[0, 0, 0, 10]),
            frame(1, 0, 1, &block[..split]),
            frame(9, 4, 1, &block[split..]),
        ]
        .concat();
        for chunk in 1..=wire.len() {
            let mut capture = Capture::default();
            for part in wire.chunks(chunk) {
                capture.feed(part).unwrap();
            }
            let request = capture.request(1).unwrap();
            assert_eq!(
                request.fingerprint().akamai(),
                "4:1;3:100;4:65535|10|0|m,a,s,p"
            );
            assert_eq!(request.fingerprint().header_order, ["x-tag", "x-tag"]);
            assert!(capture.request(3).is_none());
            capture.finish().unwrap();
        }
    }
}
#[test]
fn hpack_state_is_connection_scoped_and_snapshots_are_stream_scoped() {
    let mut first = headers();
    // Incrementally index x-tag:one as dynamic entry62.
    first.extend([0x40, 5]);
    first.extend(b"x-tag");
    first.push(3);
    first.extend(b"one");
    let mut second = headers();
    second.push(0xbe);
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    capture.feed(&frame(1, 4, 1, &first)).unwrap();
    capture.feed(&frame(1, 4, 3, &second)).unwrap();
    assert_eq!(
        capture.request(1).unwrap().fields(),
        capture.request(3).unwrap().fields()
    );
    capture.feed(&frame(3, 0, 1, &[0; 4])).unwrap();
    assert!(capture.request(1).is_none());
    assert!(capture.request(3).is_some());
}

#[test]
fn rfc7541_huffman_and_dynamic_authority_are_decoded_without_reordering() {
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    // RFC7541 AppendixC.4.1, first request with Huffman coding.
    let block = [
        0x82, 0x86, 0x84, 0x41, 0x8c, 0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90,
        0xf4, 0xff,
    ];
    capture.feed(&frame(1, 4, 1, &block)).unwrap();
    assert_eq!(
        capture.request(1).unwrap().fields()[3].1,
        b"www.example.com"
    );
    assert_eq!(
        capture.request(1).unwrap().fingerprint().pseudo_string(),
        "m,s,p,a"
    );
    capture
        .feed(&frame(1, 4, 3, &[0x82, 0x86, 0x84, 0xbe]))
        .unwrap();
    assert_eq!(
        capture.request(3).unwrap().fields()[3].1,
        b"www.example.com"
    );
}

#[test]
fn priority_uses_frozen_encoded_weight_and_separate_headers_tuple() {
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    capture.feed(&frame(2, 0, 3, &[0, 0, 0, 0, 200])).unwrap();
    let mut block = vec![0x80, 0, 0, 0, 255];
    block.extend(headers());
    capture.feed(&frame(1, 0x24, 1, &block)).unwrap();
    let fp = capture.request(1).unwrap().fingerprint();
    assert_eq!(fp.priorities, ["3:0:0:200"]);
    assert_eq!(fp.headers_priority, "1:0:255");
}

#[test]
fn settings_and_preface_cannot_be_skipped_or_replaced_by_ack() {
    for wire in [
        [PREFACE.to_vec(), frame(4, 1, 0, &[])].concat(),
        [PREFACE.to_vec(), frame(1, 4, 1, &headers())].concat(),
        vec![b'X'],
    ] {
        let mut capture = Capture::default();
        assert_eq!(capture.feed(&wire), Err(Error::Malformed));
    }
}
#[test]
fn incomplete_header_block_never_exposes_a_request_and_wrong_continuation_is_terminal() {
    for wrong in [
        frame(9, 4, 3, &[]),
        frame(6, 0, 0, &[0; 8]),
        frame(1, 4, 3, &headers()),
    ] {
        let mut capture = Capture::default();
        capture.feed(&prefix()).unwrap();
        capture.feed(&frame(1, 0, 1, &headers())).unwrap();
        assert!(capture.request(1).is_none());
        assert_eq!(capture.feed(&wrong), Err(Error::Malformed));
        assert_eq!(capture.feed(&frame(9, 4, 1, &[])), Err(Error::Malformed));
    }
}
#[test]
fn header_malformations_and_midblock_table_update_refuse() {
    let cases = [
        [headers(), literal(b"X-Test", b"a")].concat(),
        [headers(), literal(b"connection", b"close")].concat(),
        [headers(), literal(b"te", b"gzip")].concat(),
        [headers(), vec![0x82]].concat(),
        [headers(), literal(b"x-test", b"a\0b")].concat(),
        [headers(), literal(b"x-test", b" a")].concat(),
        [headers(), vec![0x20]].concat(),
        [literal(b"x-test", b"a"), headers()].concat(),
        vec![0x82, 0x87, 0x84], // missing authority
    ];
    for block in cases {
        let mut capture = Capture::default();
        capture.feed(&prefix()).unwrap();
        assert_eq!(capture.feed(&frame(1, 4, 1, &block)), Err(Error::Malformed));
        assert!(capture.request(1).is_none());
    }
}

#[test]
fn truncated_hpack_integer_and_literal_spans_refuse_without_dependency_panic() {
    for encoded in [
        vec![0xff],
        vec![0xff, 0x80],
        vec![0x3f],
        vec![0x40],
        vec![0x40, 0xff],
        vec![0x40, 1, b'x'],
        vec![0x40, 1, b'x', 0x7f],
        vec![0xff, 0x80, 0x80, 0x80, 0x80],
        vec![0, 1, b'x', 0x81, 0],
    ] {
        let mut capture = Capture::default();
        capture.feed(&prefix()).unwrap();
        assert_eq!(
            capture.feed(&frame(1, 4, 1, &encoded)),
            Err(Error::Malformed)
        );
        assert!(capture.request(1).is_none());
    }
}

#[test]
fn bounded_hostile_compression_corpus_never_publishes_partial_requests() {
    for seed in 0u32..4096 {
        let mut state = seed.wrapping_add(1);
        let mut bytes = Vec::new();
        for _ in 0..seed as usize % 65 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            bytes.push(state as u8);
        }
        let mut capture = Capture::default();
        capture.feed(&prefix()).unwrap();
        if capture.feed(&frame(1, 4, 1, &bytes)).is_err() {
            assert!(
                capture.request(1).is_none(),
                "partial request from seed {seed}"
            );
        } else {
            let request = capture.request(1).unwrap();
            assert_eq!(
                request
                    .fields()
                    .iter()
                    .filter(|(name, _)| name.starts_with(b":"))
                    .count(),
                4
            );
        }
    }
}
#[test]
fn dynamic_table_update_at_start_and_trailers_preserve_decoder_sync() {
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    let mut block = vec![0x20];
    block.extend(headers());
    capture.feed(&frame(1, 4, 1, &block)).unwrap();
    capture
        .feed(&frame(1, 5, 1, &literal(b"x-trailer", b"v")))
        .unwrap();
    assert_eq!(capture.request(1).unwrap().fields().len(), 4);
    assert_eq!(
        capture.feed(&frame(0, 1, 1, b"late")),
        Err(Error::Malformed)
    );
    assert!(capture.request(1).is_none());
}
#[test]
fn malformed_framing_and_every_truncated_prefix_refuse() {
    let wire = [prefix(), frame(1, 4, 1, &headers())].concat();
    for end in 0..wire.len() {
        let mut capture = Capture::default();
        capture.feed(&wire[..end]).unwrap();
        // EOF at the complete SETTINGS boundary is a valid empty connection.
        if end != prefix().len() {
            assert_eq!(capture.finish(), Err(Error::Truncated));
        }
        assert!(capture.request(1).is_none());
    }
    for bad in [
        frame(4, 0, 1, &[]),
        frame(4, 1, 0, &[0; 6]),
        frame(8, 0, 0, &[0; 4]),
        frame(8, 0, 3, &[0, 0, 0, 1]),
        frame(2, 0, 1, &[0, 0, 0, 1, 1]),
        frame(1, 4, 2, &headers()),
        frame(5, 0, 1, &[]),
    ] {
        let mut capture = Capture::default();
        capture.feed(&prefix()).unwrap();
        assert_eq!(capture.feed(&bad), Err(Error::Malformed));
    }
}
#[test]
fn fixed_frame_header_block_and_decoded_header_budgets_refuse() {
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    assert_eq!(
        capture.feed(&[0, 0x40, 1, 0, 0, 0, 0, 0, 1]),
        Err(Error::Budget)
    );
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    capture.feed(&frame(1, 0, 1, &[0; FRAME_LIMIT])).unwrap();
    for _ in 0..3 {
        capture.feed(&frame(9, 0, 1, &[0; FRAME_LIMIT])).unwrap();
    }
    assert_eq!(capture.feed(&frame(9, 0, 1, &[0])), Err(Error::Budget));
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    let mut block = headers();
    for _ in 0..125 {
        block.extend(literal(b"x", b"v"));
    }
    assert_eq!(capture.feed(&frame(1, 4, 1, &block)), Err(Error::Budget));
}

#[test]
fn stream_and_connection_frame_budgets_are_lifetime_limits() {
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    for stream in (1..=255).step_by(2) {
        capture.feed(&frame(1, 5, stream, &headers())).unwrap();
    }
    assert_eq!(
        capture.feed(&frame(1, 5, 257, &headers())),
        Err(Error::Budget)
    );
    assert!(capture.request(1).is_none());
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    for _ in 1..4096 {
        capture.feed(&frame(0xff, 0, 0, &[])).unwrap();
    }
    assert_eq!(capture.feed(&frame(0xff, 0, 0, &[])), Err(Error::Budget));
}

#[test]
fn frozen_monitor_is_reused_without_forking_its_thresholds() {
    let mut capture = Capture::default();
    capture.feed(&prefix()).unwrap();
    for stream in (1..=31).step_by(2) {
        capture.feed(&frame(1, 4, stream, &headers())).unwrap();
        assert_eq!(capture.abuse(), None);
        capture.feed(&frame(3, 0, stream, &[0; 4])).unwrap();
    }
    assert_eq!(capture.abuse(), Some("l5.h2dos.rapid_reset"));
}
