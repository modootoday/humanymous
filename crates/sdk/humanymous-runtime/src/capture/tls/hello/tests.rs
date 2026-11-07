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
fn with_extensions(extensions: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (kind, data) in extensions {
        bytes.extend(kind.to_be_bytes());
        bytes.extend((data.len() as u16).to_be_bytes());
        bytes.extend(data);
    }
    let mut handshake = hello();
    handshake.extend((bytes.len() as u16).to_be_bytes());
    handshake.extend(bytes);
    let length = handshake.len() - 4;
    handshake[1..4].copy_from_slice(&[(length >> 16) as u8, (length >> 8) as u8, length as u8]);
    handshake
}

#[test]
fn complete_extension_vectors_retain_wire_order_and_opaque_alpn() {
    let mut sni = vec![0, 18, 0, 0, 15];
    sni.extend(b"capture.invalid");
    let handshake = with_extensions(&[
        (43, vec![4, 3, 4, 0x7a, 0x7a]),
        (
            16,
            vec![
                0, 12, 2, b'h', b'2', 8, b'h', b't', b't', b'p', b'/', b'1', b'.', b'1',
            ],
        ),
        (10, vec![0, 4, 0x11, 0xec, 0, 0x1d]),
        (13, vec![0, 4, 4, 3, 8, 4]),
        (11, vec![1, 0]),
        (0, sni),
        (0xaaaa, vec![]),
    ]);
    let mut capture = ClientHelloCapture::default();
    for byte in record(&handshake) {
        capture.feed(&[byte]).unwrap();
    }
    let observed = capture.finish().unwrap();
    assert_eq!(observed.extensions, [43, 16, 10, 13, 11, 0, 0xaaaa]);
    assert_eq!(observed.supported_versions, [0x0304, 0x7a7a]);
    assert_eq!(observed.curves, [0x11ec, 0x001d]);
    assert_eq!(observed.signature_algorithms, [0x0403, 0x0804]);
    assert_eq!(observed.alpn, [b"h2".to_vec(), b"http/1.1".to_vec()]);
    assert!(observed.sni_present);
}

#[test]
fn duplicate_extensions_odd_vectors_and_empty_alpn_names_refuse() {
    for extensions in [
        vec![(0xaaaa, vec![]), (0xaaaa, vec![])],
        vec![(10, vec![0, 3, 0, 1, 0])],
        vec![(43, vec![3, 3, 4, 0])],
        vec![(16, vec![0, 1, 0])],
        vec![(16, vec![0, 3, 2, b'h'])],
        vec![(11, vec![1, 0, 0])],
    ] {
        let mut capture = ClientHelloCapture::default();
        assert_eq!(
            capture.feed(&record(&with_extensions(&extensions))),
            Err(CaptureError::Malformed)
        );
    }
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
    let mut capture = ClientHelloCapture {
        expected: Some(2048),
        handshake: vec![1, 0, 8, 0],
        ..Default::default()
    };
    for _ in 0..MAX_RECORDS {
        capture.feed(&record(&[0])).unwrap();
    }
    assert_eq!(capture.feed(&record(&[0])), Err(CaptureError::Budget));
}
