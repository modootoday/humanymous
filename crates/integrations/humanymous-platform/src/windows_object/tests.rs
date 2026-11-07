use super::*;

#[test]
fn unverified_wire_cannot_construct_native_observation() {
    let claim = UnverifiedIdentityClaim::from_wire([42; 24]);
    assert_eq!(claim.to_wire(), [42; 24]);
    // No native observation/verified carrier constructor is available from claim.
}

#[test]
fn volume_form_rejects_unc_dos_device_and_malformed_guid() {
    let valid = r"\\?\Volume{12345678-1234-1234-abcd-1234567890ab}\carrier\context.json";
    assert_eq!(
        query::volume_prefix(valid).unwrap(),
        r"\\?\Volume{12345678-1234-1234-abcd-1234567890ab}\"
    );
    for bad in [
        r"C:\carrier",
        r"\\server\share",
        r"\\?\UNC\server\share",
        r"\\.\Device",
        r"\\?\Volume{123456781234-1234-abcd-1234567890ab}\x",
        r"\\?\Volume{12345678-1234-1234-abcd-1234567890az}\x",
    ] {
        assert!(query::volume_prefix(bad).is_err());
    }
}

#[test]
fn utf16_and_required_capacity_are_bounded_including_terminator() {
    assert_eq!(
        create::wide_path(Path::new("abc")).unwrap(),
        [97, 98, 99, 0]
    );
    assert!(create::wide_path(Path::new("")).is_err());
    assert!(create::wide_path(Path::new("a\0b")).is_err());
    assert!(create::wide_path(Path::new(&"a".repeat(32768))).is_err());
    assert_eq!(query::check_capacity(256, 32768).unwrap(), 32768);
    assert_eq!(query::check_capacity(256, 256).unwrap(), 257);
    assert!(query::check_capacity(256, 32769).is_err());
    assert!(query::check_capacity(usize::MAX, usize::MAX).is_err());
}

#[test]
fn utf16_code_unit_limit_includes_final_terminator() {
    let ascii_max = "a".repeat(32767);
    assert_eq!(
        create::wide_path(Path::new(&ascii_max)).unwrap().len(),
        32768
    );
    assert!(create::wide_path(Path::new(&"a".repeat(32768))).is_err());
    let surrogate_max = format!("{}\u{1f642}", "a".repeat(32765));
    assert_eq!(
        create::wide_path(Path::new(&surrogate_max)).unwrap().len(),
        32768
    );
    let surrogate_over = format!("{}\u{1f642}", "a".repeat(32766));
    assert!(create::wide_path(Path::new(&surrogate_over)).is_err());
}
