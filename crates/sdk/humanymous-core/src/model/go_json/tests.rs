//! Goldens observed 2026-11-18 from go1.25.3 linux/amd64 `encoding/json`.
use super::*;

#[derive(Serialize)]
struct Probe {
    a: f32,
    b: f64,
    xs: Vec<f32>,
    ys: Vec<f64>,
    t: &'static str,
}

#[test]
fn compact_numbers_and_strings_match_go_encoding_json() {
    let probe = Probe {
        a: 1.0,
        b: 0.0,
        xs: vec![
            0.0,
            1.0,
            0.1,
            1e-6,
            9.99e-7,
            1e-7,
            1.5e-8,
            123_456.7,
            1e20,
            1e21,
            3.4e38,
            -2.5e-10,
            0.333_333_34,
            16_777_216.0,
        ],
        ys: vec![
            0.1,
            1e-7,
            1e21,
            1.2345e22,
            5e-324,
            123_456_789_012_345_680.0,
            0.6,
            1e-6,
        ],
        t: "a<b>&c\u{2028}d\u{2029}e\"f\\g\n",
    };
    let go = concat!(
        r#"{"a":1,"b":0,"xs":[0,1,0.1,0.000001,9.99e-7,1e-7,1.5e-8,123456.7,"#,
        r#"100000000000000000000,1e+21,3.4e+38,-2.5e-10,0.33333334,16777216],"#,
        r#""ys":[0.1,1e-7,1e+21,1.2345e+22,5e-324,123456789012345680,0.6,0.000001],"#,
        r#""t":"a\u003cb\u003e\u0026c\u2028d\u2029e\"f\\g\n"}"#
    );
    assert_eq!(String::from_utf8(to_vec(&probe).unwrap()).unwrap(), go);
}

#[test]
fn indented_layout_matches_go_marshal_indent() {
    let value = serde_json::json!({ "a": {}, "m": [], "z": [1, 2] });
    assert_eq!(
        String::from_utf8(to_vec_indent(&value, b" ").unwrap()).unwrap(),
        "{\n \"a\": {},\n \"m\": [],\n \"z\": [\n  1,\n  2\n ]\n}"
    );
}

#[test]
fn float_text_helpers_follow_go_exponent_cutoffs() {
    assert_eq!(go_f32(1e-6), "0.000001");
    assert_eq!(go_f32(9.99e-7), "9.99e-7");
    assert_eq!(go_f32(-1e21), "-1e+21");
    assert_eq!(go_f64(0.0), "0");
    assert_eq!(go_f64(-0.0), "-0");
    assert_eq!(go_f64(2.5), "2.5");
}
