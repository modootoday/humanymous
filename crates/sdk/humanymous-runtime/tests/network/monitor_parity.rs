//! Frame completion and threshold timing against the actual frozen Go monitor.
use humanymous_runtime::network::http2::FrameMonitor;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    input: String,
    chunk_size: usize,
    expected: Vec<String>,
}

#[test]
fn passive_monitor_matches_go_for_every_chunk_and_threshold() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/network/monitor.json")).unwrap();
    assert_eq!(cases.len(), 2520);
    for case in cases {
        let input = hex::decode(case.input).unwrap();
        let mut monitor = FrameMonitor::default();
        let chunks = input.chunks(case.chunk_size).collect::<Vec<_>>();
        assert_eq!(chunks.len(), case.expected.len());
        for (bytes, expected) in chunks.into_iter().zip(case.expected) {
            monitor.feed(bytes);
            assert_eq!(monitor.abuse().unwrap_or(""), expected);
        }
    }
}
