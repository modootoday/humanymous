//! Smuggle-classification wargame rounds from legacy internal/gate
//! wargame_escalation_r81 and r161 (baseline 981d25c). Go `smuggleBadTE`,
//! `smuggleDupCL`, `smuggleTECL`, `smuggleObsFold` and `smuggleNone` are
//! `TeNotChunked`, `AmbiguousLength`, `TeClConflict`, `ObsFold` and `None`.
//! These pin the classifier only; the Rust HTTP parser's own framing
//! refusal (including the lone chunked TE difference, R169) is separate.
use super::*;

/// Go TestWargameR096_SmuggleTEIdentity.
#[test]
fn wargame_r096_smuggle_te_identity() {
    assert_eq!(
        scan(&h(&[("Transfer-Encoding", "identity")])),
        Some(Reason::TeNotChunked)
    );
}

/// Go TestWargameR097_SmuggleTEGzipChunkedList.
#[test]
fn wargame_r097_smuggle_te_gzip_chunked_list() {
    assert_eq!(
        scan(&h(&[("Transfer-Encoding", "gzip, chunked")])),
        Some(Reason::TeNotChunked)
    );
}

/// Go TestWargameR098_SmuggleObsFoldLFOnly.
#[test]
fn wargame_r098_smuggle_obs_fold_lf_only() {
    assert_eq!(scan(&h(&[("X-Foo", "a\nb")])), Some(Reason::ObsFold));
}

/// Go TestWargameR168_SmuggleAndBanIndependent: differing duplicate lengths.
#[test]
fn wargame_r168_smuggle_and_ban_independent() {
    assert_eq!(
        scan(&h(&[("Content-Length", "1"), ("Content-Length", "2")])),
        Some(Reason::AmbiguousLength)
    );
}

/// Go TestWargameR169_TEChunkedAloneOK (classifier level).
#[test]
fn wargame_r169_te_chunked_alone_ok() {
    assert_eq!(scan(&h(&[("Transfer-Encoding", "chunked")])), None);
}

/// Go TestWargameR170_CLAloneOK.
#[test]
fn wargame_r170_cl_alone_ok() {
    assert_eq!(scan(&h(&[("Content-Length", "10")])), None);
}

/// Go TestWargameR197_SmuggleCRInHeaderValue.
#[test]
fn wargame_r197_smuggle_cr_in_header_value() {
    assert_eq!(scan(&h(&[("X-Foo", "a\rb")])), Some(Reason::ObsFold));
}

/// Go TestWargameR198_MultiHeaderObsFoldAnyKey.
#[test]
fn wargame_r198_multi_header_obs_fold_any_key() {
    assert_eq!(
        scan(&h(&[("X-Other", "safe"), ("X-Bad", "line1\nline2")])),
        Some(Reason::ObsFold)
    );
}
