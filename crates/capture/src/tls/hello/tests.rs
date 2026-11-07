use super::*;

fn hello() -> Vec<u8> {
    let mut body = vec![3, 3];
    body.extend([0; 32]);
    body.extend([0, 0, 4, 0x0a, 0x0a, 0x13, 1, 1, 0]);
    let mut result = vec![1, 0, 0, body.len() as u8];
    result.extend(body);
    result
}
fn record(bytes: &[u8]) -> Vec<u8> {
    let mut result = vec![22, 3, 1, (bytes.len() >> 8) as u8, bytes.len() as u8];
    result.extend(bytes);
    result
}
#[test]
fn every_chunk_and_record_split_preserves_the_same_hello() {
    let handshake = hello();
    for split in 1..handshake.len() {
        let wire = [record(&handshake[..split]), record(&handshake[split..])].concat();
        for chunk in 1..=wire.len() {
            let mut capture = ClientHelloCapture::default();
            for part in wire.chunks(chunk) {
                capture.feed(part).unwrap();
            }
            assert!(capture.is_complete());
            let observed = capture.finish().unwrap();
            assert_eq!(observed.cipher_suites, [0x0a0a, 0x1301]);
        }
    }
}
#[test]
fn every_truncated_wire_prefix_refuses_instead_of_using_tolerant_parser() {
    let wire = record(&hello());
    for end in 0..wire.len() {
        let mut capture = ClientHelloCapture::default();
        capture.feed(&wire[..end]).unwrap();
        assert_eq!(capture.finish(), Err(CaptureError::Truncated));
    }
}
#[test]
fn malformed_and_over_budget_inputs_are_terminal() {
    for (wire, expected) in [
        (vec![22, 3, 3, 0xff, 0xff], CaptureError::Budget),
        (vec![23, 3, 3, 0, 1, 1], CaptureError::Malformed),
        (record(&[1, 1, 0, 0]), CaptureError::Budget),
        (record(&[2, 0, 0, 0]), CaptureError::Malformed),
        (
            record(&[hello(), vec![0]].concat()),
            CaptureError::Malformed,
        ),
    ] {
        let mut capture = ClientHelloCapture::default();
        assert_eq!(capture.feed(&wire), Err(expected));
        assert_eq!(capture.feed(&record(&hello())), Err(expected));
        assert_eq!(capture.finish(), Err(expected));
    }
}
#[test]
fn nested_truncation_and_odd_cipher_list_are_not_complete() {
    let mut handshake = hello();
    handshake[40] = 3;
    let mut capture = ClientHelloCapture::default();
    assert_eq!(
        capture.feed(&record(&handshake)),
        Err(CaptureError::Malformed)
    );
    let mut handshake = hello();
    handshake.extend([0, 5, 0, 16, 0, 3, 0]);
    handshake[3] = (handshake.len() - 4) as u8;
    let mut capture = ClientHelloCapture::default();
    assert_eq!(
        capture.feed(&record(&handshake)),
        Err(CaptureError::Malformed)
    );
}
#[test]
fn excessive_fragment_records_fail_with_constant_count_budget() {
    let mut capture = ClientHelloCapture::default();
    capture.expected = Some(2048);
    capture.handshake = vec![1, 0, 8, 0];
    for _ in 0..MAX_RECORDS {
        capture.feed(&record(&[0])).unwrap();
    }
    assert_eq!(capture.feed(&record(&[0])), Err(CaptureError::Budget));
}
