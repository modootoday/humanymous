use super::*;

fn limits() -> Limits {
    Limits::new(MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, MAX_CANDIDATES).unwrap()
}

#[test]
fn independent_ieee_crc_and_exact_frame_bytes() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(crc32(b""), 0);
    assert_eq!(
        encode(b"123456789", limits()).unwrap(),
        b"HMWM\0\x09123456789\xcb\xf4\x39\x26"
    );
    assert_eq!(
        recover(b"noiseHMWM\0\0\0\0\0\0suffix", limits()).unwrap(),
        b""
    );
}

#[test]
fn noise_damage_duplicates_and_distinct_carriers() {
    let a = encode(b"alpha", limits()).unwrap();
    let b = encode(b"beta", limits()).unwrap();
    let mut bad = a.clone();
    *bad.last_mut().unwrap() ^= 1;
    let mut data = bad;
    data.extend_from_slice(&a);
    assert_eq!(recover(&data, limits()).unwrap(), b"alpha");
    data.extend_from_slice(&a);
    assert_eq!(recover(&data, limits()).unwrap(), b"alpha");
    data.extend_from_slice(&b);
    assert_eq!(recover(&data, limits()), Err(Error::Ambiguous));
}

#[test]
fn exact_carrier_limit_truncation_and_budget_fail_closed() {
    let carrier = vec![1; MAX_CARRIER_BYTES];
    let frame = encode(&carrier, limits()).unwrap();
    assert_eq!(recover(&frame, limits()).unwrap(), carrier);
    assert_eq!(
        encode(&vec![1; MAX_CARRIER_BYTES + 1], limits()),
        Err(Error::ResourceLimit)
    );
    for size in 0..10 {
        assert_eq!(recover(&frame[..size], limits()), Err(Error::NoFrame));
    }
    let tiny = Limits::new(32, 64, 1).unwrap();
    let a = encode(b"a", tiny).unwrap();
    let mut two = a.clone();
    two.extend_from_slice(&a);
    assert_eq!(recover(&two, tiny), Err(Error::ResourceLimit));
    assert_eq!(recover(&[1; 33], tiny), Err(Error::ResourceLimit));
    assert_eq!(
        encode(b"a", Limits::new(32, 10, 1).unwrap()),
        Err(Error::ResourceLimit)
    );
}

#[test]
fn configurations_cannot_widen_accepted_admission_caps() {
    for (input, output, candidates) in [
        (0, 1, 1),
        (1, 0, 1),
        (1, 1, 0),
        (MAX_INPUT_BYTES + 1, 1, 1),
        (1, MAX_OUTPUT_BYTES + 1, 1),
        (1, 1, MAX_CANDIDATES + 1),
    ] {
        assert!(matches!(
            Limits::new(input, output, candidates),
            Err(Error::InvalidConfiguration)
        ));
    }
}

#[test]
fn repeated_valid_frames_still_consume_work_and_cannot_hide_overflow() {
    let frame = encode(b"same", limits()).unwrap();
    let mut data = frame.repeat(MAX_CANDIDATES);
    assert_eq!(recover(&data, limits()).unwrap(), b"same");
    data.extend_from_slice(&frame);
    assert_eq!(recover(&data, limits()), Err(Error::ResourceLimit));
}
