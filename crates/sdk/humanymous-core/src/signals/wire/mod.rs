//! Cross-language wire compatibility shared by every contract consumer.
pub mod decode;
use serde::Deserialize;
use std::fmt::Write;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct UnicodeData {
    version: String,
    lower: Vec<[u32; 2]>,
    printable: Vec<[u32; 2]>,
}

fn unicode() -> &'static UnicodeData {
    static DATA: OnceLock<UnicodeData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("unicode.json")).expect("validated frozen Unicode tables")
    })
}

/// Unicode edition frozen by the source wire contract.
pub fn unicode_version() -> &'static str {
    &unicode().version
}

/// Quote a UTF-8 string with Go encoding/json's default HTML and line-separator
/// escaping. This does not claim general JSON number/struct canonicalization.
pub fn quote_json_go(value: &str) -> String {
    let mut output = Vec::new();
    quote_json_go_to(value, &mut output).expect("vector write cannot fail");
    String::from_utf8(output).expect("quoted UTF-8 remains UTF-8")
}

/// Write the same Go-compatible JSON string without allocating an escaped copy.
/// A sink error is returned immediately; partial output must not be committed.
pub fn quote_json_go_to(value: &str, output: &mut impl std::io::Write) -> std::io::Result<()> {
    output.write_all(b"\"")?;
    for character in value.chars() {
        let escaped = match character {
            '"' => Some("\\\""),
            '\\' => Some("\\\\"),
            '\u{8}' => Some("\\b"),
            '\u{c}' => Some("\\f"),
            '\n' => Some("\\n"),
            '\r' => Some("\\r"),
            '\t' => Some("\\t"),
            '<' => Some("\\u003c"),
            '>' => Some("\\u003e"),
            '&' => Some("\\u0026"),
            '\u{2028}' => Some("\\u2028"),
            '\u{2029}' => Some("\\u2029"),
            _ => None,
        };
        if let Some(escaped) = escaped {
            output.write_all(escaped.as_bytes())?;
        } else if character <= '\u{1f}' {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let code = character as usize;
            output.write_all(&[b'\\', b'u', b'0', b'0', HEX[code >> 4], HEX[code & 15]])?;
        } else {
            output.write_all(character.encode_utf8(&mut [0; 4]).as_bytes())?;
        }
    }
    output.write_all(b"\"")
}

/// Represent a float32 payload using its shortest round-trip decimal, as Go's
/// JSON encoder does for float32 interface values. Nonfinite values become null,
/// matching serde_json's float serialization; valid model outputs are finite.
pub fn float32_json_value(value: f32) -> serde_json::Value {
    if !value.is_finite() {
        return serde_json::Value::Null;
    }
    serde_json::from_str(&value.to_string()).expect("finite float32 JSON number")
}

/// Shortest round-trip decimal digits of a finite value as Go 1.26.3
/// `strconv` selects them for precision -1: sign, significant digits without
/// leading zeros (empty for zero) and the decimal point position, so that the
/// magnitude is `0.digits * 10^point`. Among equally short candidates Go keeps
/// the one nearest the binary value and breaks an exact tie toward an even
/// last digit; Rust's own shortest formatter does not, so Go's Dragonbox
/// selection is ported in [`go_shortest`].
fn shortest_digits(value: f64) -> (bool, Vec<u8>, i32) {
    let bits = value.to_bits();
    let negative = bits >> 63 != 0;
    let biased = i32::try_from((bits >> 52) & 0x7ff).expect("11-bit exponent");
    let mut mantissa = bits & ((1 << 52) - 1);
    let denormal = biased == 0;
    if denormal && mantissa == 0 {
        return (negative, Vec::new(), 0);
    }
    let mut exponent = biased;
    if denormal {
        exponent += 1;
    } else {
        mantissa |= 1 << 52;
    }
    exponent -= 1023;
    let (significand, decimal_exponent) = go_shortest::shortest(mantissa, exponent - 52, denormal);
    let digits = significand.to_string().into_bytes();
    let point = i32::try_from(digits.len()).expect("at most 20 digits") + decimal_exponent;
    (negative, digits, point)
}

/// Port of the float64 shortest path of Go 1.26.3 `internal/strconv`
/// (`ftoadbox.go` `dboxFtoa64` with its `math.go` helpers and the
/// `pow10gen.go` table definition), the Dragonbox algorithm by Junekey Jeon.
/// Copyright 2025 The Go Authors, BSD-3-Clause; the notice is retained in
/// `crates/sdk/humanymous-core/src/model/training/rng/LICENSE`. Integer
/// arithmetic keeps Go's wrapping and out-of-range shift semantics.
mod go_shortest {
    use std::sync::OnceLock;

    const MANTISSA_BITS: i32 = 52;
    const POW10_MIN: i32 = -348;
    const POW10_MAX: i32 = 347;

    fn hi(value: u128) -> u64 {
        (value >> 64) as u64
    }

    fn lo(value: u128) -> u64 {
        value as u64
    }

    /// Go `x >> s` for unsigned x: shifts of the full width or more give 0.
    fn shr(value: u64, shift: i32) -> u64 {
        u32::try_from(shift)
            .ok()
            .and_then(|shift| value.checked_shr(shift))
            .unwrap_or(0)
    }

    /// Go `x << s` for unsigned x: shifts of the full width or more give 0.
    fn shl(value: u64, shift: i32) -> u64 {
        u32::try_from(shift)
            .ok()
            .and_then(|shift| value.checked_shl(shift))
            .unwrap_or(0)
    }

    /// Floor of `x * log10(2)` for |x| <= 1600 (Go `mulLog10_2`).
    fn mul_log10_2(x: i32) -> i32 {
        (x * 78913) >> 18
    }

    /// Floor of `x * log2(10)` for |x| <= 500 (Go `mulLog2_10`).
    fn mul_log2_10(x: i32) -> i32 {
        (x * 108_853) >> 15
    }

    /// Floor of `x * log10(2) - log10(4/3)` (Go `mulLog10_2MinusLog10_4Over3`).
    fn mul_log10_2_minus_log10_4_over_3(x: i32) -> i32 {
        (x * 631_305 - 261_663) >> 21
    }

    /// Little-endian 64-bit limbs of an unsigned integer.
    fn bit_length(limbs: &[u64]) -> u32 {
        limbs
            .iter()
            .rposition(|limb| *limb != 0)
            .map_or(0, |index| {
                64 * index as u32 + 64 - limbs[index].leading_zeros()
            })
    }

    /// The 128 bits of `limbs` starting at bit `low` (bits beyond are zero).
    fn window(limbs: &[u64], low: u32) -> u128 {
        let mut result = 0_u128;
        for bit in 0..128 {
            let index = low + bit;
            let limb = limbs.get((index / 64) as usize).copied().unwrap_or(0);
            if (limb >> (index % 64)) & 1 == 1 {
                result |= 1 << bit;
            }
        }
        result
    }

    /// Go `pow10Tab`: for 10^e the 128-bit mantissa floor(10^e * 2^b) with
    /// its top bit set. Floors of successive exact divisions compose, so the
    /// negative powers are exact floors of 2^(127 + bitlen(10^-e)) / 10^-e.
    fn pow10_table() -> &'static [u128] {
        static TABLE: OnceLock<Vec<u128>> = OnceLock::new();
        TABLE.get_or_init(|| {
            (POW10_MIN..=POW10_MAX)
                .map(|exponent| {
                    let mut power = vec![1_u64];
                    for _ in 0..exponent.unsigned_abs() {
                        let mut carry = 0_u128;
                        for limb in &mut power {
                            let product = u128::from(*limb) * 10 + carry;
                            *limb = product as u64;
                            carry = product >> 64;
                        }
                        if carry != 0 {
                            power.push(carry as u64);
                        }
                    }
                    let length = bit_length(&power);
                    if exponent >= 0 {
                        return if length <= 128 {
                            window(&power, 0) << (128 - length)
                        } else {
                            window(&power, length - 128)
                        };
                    }
                    let shift = 127 + length;
                    let mut quotient = vec![0_u64; shift as usize / 64 + 1];
                    quotient[shift as usize / 64] = 1 << (shift % 64);
                    for _ in 0..exponent.unsigned_abs() {
                        let mut remainder = 0_u128;
                        for limb in quotient.iter_mut().rev() {
                            let current = (remainder << 64) | u128::from(*limb);
                            *limb = (current / 10) as u64;
                            remainder = current % 10;
                        }
                    }
                    window(&quotient, 0)
                })
                .collect()
        })
    }

    /// Go `pow10`: mantissa and binary exponent with 10^e = mant/2^128 * 2^exp.
    fn pow10(exponent: i32) -> (u128, i32) {
        let index = usize::try_from(exponent - POW10_MIN).expect("power of ten in table range");
        (pow10_table()[index], 1 + mul_log2_10(exponent))
    }

    /// Go `trimZeros`: largest p with x % 10^p == 0, returning (x / 10^p, p).
    fn trim_zeros(mut value: u64) -> (u64, i32) {
        const DIV_1E8: u64 = 0xc767_074b_22e9_0e21;
        const DIV_1E4: u64 = 0xd288_ce70_3afb_7e91;
        const DIV_1E2: u64 = 0x8f5c_28f5_c28f_5c29;
        const DIV_1E1: u64 = 0xcccc_cccc_cccc_cccd;
        let mut zeros = 0;
        loop {
            let quotient = value.wrapping_mul(DIV_1E8).rotate_right(8);
            if quotient > u64::MAX / 100_000_000 {
                break;
            }
            value = quotient;
            zeros += 8;
        }
        for (inverse, limit, rotation, count) in [
            (DIV_1E4, u64::MAX / 10_000, 4, 4),
            (DIV_1E2, u64::MAX / 100, 2, 2),
            (DIV_1E1, u64::MAX / 10, 1, 1),
        ] {
            let quotient = value.wrapping_mul(inverse).rotate_right(rotation);
            if quotient <= limit {
                value = quotient;
                zeros += count;
            }
        }
        (value, zeros)
    }

    /// Go `dboxPow64`: the precomputed phi~k and beta for float64.
    fn pow64(k: i32, exponent: i32) -> (u128, i32) {
        let (mut phi, binary) = pow10(k);
        if !(0..=55).contains(&k) {
            phi = (u128::from(hi(phi)) << 64) | u128::from(lo(phi).wrapping_add(1));
        }
        (phi, exponent + binary - 1)
    }

    /// Go `umul192Upper128`: upper 128 of the 192-bit product.
    fn mul_upper(x: u64, y: u128) -> u128 {
        u128::from(x) * u128::from(hi(y)) + ((u128::from(x) * u128::from(lo(y))) >> 64)
    }

    /// Go `dboxMulPow64`: integer part and whether the fraction is zero.
    fn mul_pow(u: u64, phi: u128) -> (u64, bool) {
        let product = mul_upper(u, phi);
        (hi(product), lo(product) == 0)
    }

    /// Go `dboxParity64`: parity of the integer part and fraction-is-zero.
    fn parity(mantissa2: u64, phi: u128, beta: i32) -> (bool, bool) {
        let product = u128::from(mantissa2).wrapping_mul(phi);
        let parity = shr(hi(product), 64 - beta) & 1 != 0;
        let integer = shl(hi(product), beta) | shr(lo(product), 64 - beta) == 0;
        (parity, integer)
    }

    /// Go `dboxDelta64`.
    fn delta(phi: u128, beta: i32) -> u32 {
        shr(hi(phi), 64 - 1 - beta) as u32
    }

    /// Go `dboxRange64`: left and right endpoints of the shorter interval.
    fn range(phi: u128, beta: i32) -> (u64, u64) {
        let high = hi(phi);
        let shift = 64 - MANTISSA_BITS - 1 - beta;
        let left = shr(high.wrapping_sub(high >> (MANTISSA_BITS + 2)), shift);
        let right = shr(high.wrapping_add(high >> (MANTISSA_BITS + 1)), shift);
        (left, right)
    }

    /// Go `dboxRoundUp64`.
    fn round_up(phi: u128, beta: i32) -> u64 {
        shr(hi(phi), 64 - MANTISSA_BITS - 2 - beta).wrapping_add(1) / 2
    }

    /// Go `dboxFtoa64` for a nonzero significand `mantissa` (implicit bit
    /// included) and binary exponent: the decimal significand and exponent
    /// whose digits Go's `dboxDigits` emits (`point = digits + exponent`).
    pub(super) fn shortest(mantissa: u64, exponent: i32, denormal: bool) -> (u64, i32) {
        if mantissa == 1 << MANTISSA_BITS && !denormal {
            let k0 = -mul_log10_2_minus_log10_4_over_3(exponent);
            let (phi, beta) = pow64(k0, exponent);
            let (mut left, right) = range(phi, beta);
            if exponent != 2 && exponent != 3 {
                left = left.wrapping_add(1);
            }
            let quotient = right / 10;
            if left <= quotient.wrapping_mul(10) {
                let (quotient, zeros) = trim_zeros(quotient);
                return (quotient, -k0 + 1 + zeros);
            }
            let mut rounded = round_up(phi, beta);
            if exponent == -77 && !rounded.is_multiple_of(2) {
                rounded = rounded.wrapping_sub(1);
            } else if rounded < left {
                rounded = rounded.wrapping_add(1);
            }
            return (rounded, -k0);
        }

        const KAPPA: i32 = 2;
        const P10_KAPPA: u32 = 100;
        const P10_KAPPA1: u64 = 1000;

        let k0 = -mul_log10_2(exponent);
        let (phi, beta) = pow64(KAPPA + k0, exponent);
        let (upper, exact) = mul_pow(shl(mantissa.wrapping_mul(2).wrapping_add(1), beta), phi);
        let mut significand = upper / P10_KAPPA1;
        let mut remainder = (upper % P10_KAPPA1) as u32;
        let delta = delta(phi, beta);

        if remainder < delta {
            if remainder != 0 || !exact || mantissa.is_multiple_of(2) {
                let (significand, zeros) = trim_zeros(significand);
                return (significand, -k0 + 1 + zeros);
            }
            significand = significand.wrapping_sub(1);
            remainder = P10_KAPPA * 10;
        } else if remainder == delta {
            let (odd, exact) = parity(mantissa.wrapping_mul(2).wrapping_sub(1), phi, beta);
            if odd || (exact && mantissa.is_multiple_of(2)) {
                let (significand, zeros) = trim_zeros(significand);
                return (significand, -k0 + 1 + zeros);
            }
        }

        let distance = remainder
            .wrapping_add(P10_KAPPA / 2)
            .wrapping_sub(delta / 2);
        let (tens, rest) = (distance / P10_KAPPA, distance % P10_KAPPA);
        let mut rounded = significand.wrapping_mul(10).wrapping_add(u64::from(tens));
        if rest == 0 {
            let (odd, exact) = parity(mantissa.wrapping_mul(2), phi, beta);
            if odd != !distance.wrapping_sub(P10_KAPPA / 2).is_multiple_of(2)
                || (exact && !rounded.is_multiple_of(2))
            {
                rounded = rounded.wrapping_sub(1);
            }
        }
        (rounded, -k0)
    }

    #[cfg(test)]
    pub(super) fn table() -> &'static [u128] {
        pow10_table()
    }
}

/// Go strconv `%e` layout of shortest digits with `precision` fraction digits.
fn go_exponent(output: &mut String, digits: &[u8], point: i32, precision: usize) {
    output.push(char::from(*digits.first().unwrap_or(&b'0')));
    if precision > 0 {
        output.push('.');
        for index in 1..=precision {
            output.push(char::from(*digits.get(index).unwrap_or(&b'0')));
        }
    }
    let exponent = if digits.is_empty() { 0 } else { point - 1 };
    output.push('e');
    output.push(if exponent < 0 { '-' } else { '+' });
    let magnitude = exponent.unsigned_abs();
    if magnitude < 10 {
        output.push('0');
    }
    write!(output, "{magnitude}").expect("string write");
}

/// Go strconv `%f` layout of shortest digits with `precision` fraction digits.
fn go_fixed(output: &mut String, digits: &[u8], point: i32, precision: usize) {
    if point > 0 {
        for index in 0..point as usize {
            output.push(char::from(*digits.get(index).unwrap_or(&b'0')));
        }
    } else {
        output.push('0');
    }
    if precision > 0 {
        output.push('.');
        for index in 1..=precision as i32 {
            let position = point + index - 1;
            let digit = usize::try_from(position)
                .ok()
                .and_then(|position| digits.get(position))
                .copied()
                .unwrap_or(b'0');
            output.push(char::from(digit));
        }
    }
}

fn go_nonfinite(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some("NaN")
    } else if value == f64::INFINITY {
        Some("+Inf")
    } else if value == f64::NEG_INFINITY {
        Some("-Inf")
    } else {
        None
    }
}

/// Format a float64 like Go `fmt` `%v` (`strconv.FormatFloat(v, 'g', -1, 64)`):
/// shortest digits, in exponent form when the decimal exponent is below -4 or
/// at least 6 (Go's `%g` threshold for shortest precision), else fixed form.
pub fn format_float_go(value: f64) -> String {
    if let Some(text) = go_nonfinite(value) {
        return text.into();
    }
    let (negative, digits, point) = shortest_digits(value);
    let mut output = String::new();
    if negative {
        output.push('-');
    }
    let exponent = point - 1;
    if !digits.is_empty() && !(-4..6).contains(&exponent) {
        go_exponent(&mut output, &digits, point, digits.len() - 1);
    } else {
        let precision = usize::try_from(digits.len() as i32 - point).unwrap_or(0);
        go_fixed(&mut output, &digits, point, precision);
    }
    output
}

/// Format a float64 like Go `fmt` `%.Nf`, including Go's non-finite spellings.
pub fn format_fixed_go(value: f64, precision: usize) -> String {
    match go_nonfinite(value) {
        Some(text) => text.into(),
        None => format!("{value:.precision$}"),
    }
}

/// Encode a float64 as Go `encoding/json` does. Go refuses NaN and infinities
/// with an unsupported-value error, so they yield `None` here.
pub fn json_float_go(value: f64) -> Option<String> {
    if go_nonfinite(value).is_some() {
        return None;
    }
    let (negative, digits, point) = shortest_digits(value);
    let mut output = String::new();
    if negative {
        output.push('-');
    }
    let magnitude = value.abs();
    if magnitude != 0.0 && !(1e-6..1e21).contains(&magnitude) {
        go_exponent(&mut output, &digits, point, digits.len() - 1);
        // Go trims one leading zero from a negative two-digit exponent.
        if let Some(index) = output.rfind("e-0") {
            output.remove(index + 2);
        }
    } else {
        let precision = usize::try_from(digits.len() as i32 - point).unwrap_or(0);
        go_fixed(&mut output, &digits, point, precision);
    }
    Some(output)
}

/// Unicode simple lowercase used by the frozen Go wire contracts.
/// Unlike full string lowercase, this neither expands dotted I nor applies
/// contextual final sigma mappings.
pub fn simple_lowercase(value: &str) -> String {
    if value.is_ascii() {
        return value.to_ascii_lowercase();
    }
    let mappings = &unicode().lower;
    value
        .chars()
        .map(|character| {
            mappings
                .binary_search_by_key(&(character as u32), |pair| pair[0])
                .ok()
                .and_then(|index| char::from_u32(mappings[index][1]))
                .unwrap_or(character)
        })
        .collect()
}

/// Quote a valid UTF-8 string with the frozen Go `%q` diagnostic escaping.
pub fn quote_go(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{7}' => result.push_str("\\a"),
            '\u{8}' => result.push_str("\\b"),
            '\u{c}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{b}' => result.push_str("\\v"),
            _ => {
                let code = character as u32;
                let ranges = &unicode().printable;
                let index = ranges.partition_point(|range| range[0] <= code);
                if index > 0 && code <= ranges[index - 1][1] {
                    result.push(character);
                } else if code < 32 || code == 127 {
                    write!(result, "\\x{code:02x}").expect("string write");
                } else if code < 65536 {
                    write!(result, "\\u{code:04x}").expect("string write");
                } else {
                    write!(result, "\\U{code:08x}").expect("string write");
                }
            }
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_json_matches_prior_encoder_for_control_and_unicode_ranges() {
        let text: String = (0..=0x10ffff).filter_map(char::from_u32).collect();
        let expected = serde_json::to_string(&text)
            .unwrap()
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026")
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029");
        assert_eq!(quote_json_go(&text), expected);
        let mut bytes = Vec::new();
        quote_json_go_to(&text, &mut bytes).unwrap();
        assert_eq!(bytes, expected.as_bytes());
    }

    #[test]
    fn streaming_json_propagates_partial_sink_failure() {
        struct Limited(Vec<u8>);
        impl std::io::Write for Limited {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                let remaining = 4_usize.saturating_sub(self.0.len());
                if remaining == 0 {
                    return Err(std::io::Error::other("injected sink failure"));
                }
                let count = remaining.min(bytes.len());
                self.0.extend_from_slice(&bytes[..count]);
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut sink = Limited(Vec::new());
        assert!(quote_json_go_to("<secret", &mut sink).is_err());
        assert_eq!(sink.0, b"\"\\u0");
    }
    #[test]
    fn unicode_simple_mapping_does_not_expand_or_depend_on_context() {
        assert_eq!(simple_lowercase("İ ΟΣ User-Agent"), "i οσ user-agent");
    }
    #[test]
    fn json_string_escaping_preserves_go_signed_byte_contracts() {
        assert_eq!(
            quote_json_go("<&>\u{2028}\u{2029}"),
            "\"\\u003c\\u0026\\u003e\\u2028\\u2029\""
        );
        assert_eq!(quote_json_go("é\n\"\\\0"), "\"é\\n\\\"\\\\\\u0000\"");
        assert_eq!(quote_json_go("\\u003c"), "\"\\\\u003c\"");
    }
    #[test]
    fn tables_and_quoting_match_frozen_unicode_contract() {
        let data = unicode();
        assert_eq!(data.version, "15.0.0");
        assert!(data.lower.windows(2).all(|pair| pair[0][0] < pair[1][0]));
        assert!(
            data.printable
                .windows(2)
                .all(|pair| pair[0][1] < pair[1][0])
        );
        #[derive(Deserialize)]
        struct Case {
            input: String,
            lower: String,
            quoted: String,
        }
        #[derive(Deserialize)]
        struct Corpus {
            cases: Vec<Case>,
        }
        let corpus: Corpus = serde_json::from_str(include_str!("unicode.json")).unwrap();
        assert_eq!(corpus.cases.len(), 8);
        for case in corpus.cases {
            assert_eq!(simple_lowercase(&case.input), case.lower);
            assert_eq!(quote_go(&case.input), case.quoted);
        }
    }
    /// Values captured from Go 1.26.3 `%v`, `%.1f` and `json.Marshal` at the
    /// frozen oracle commit 981d25c (settings overlay golden capture).
    #[test]
    fn float_formatting_matches_go_fmt_and_encoding_json() {
        for (value, v, fixed, json) in [
            (0.0, "0", "0.0", Some("0")),
            (-0.0, "-0", "-0.0", Some("-0")),
            (-1.0, "-1", "-1.0", Some("-1")),
            (2.5, "2.5", "2.5", Some("2.5")),
            (29.95, "29.95", "29.9", Some("29.95")),
            (60.05, "60.05", "60.0", Some("60.05")),
            (60.25, "60.25", "60.2", Some("60.25")),
            (0.05, "0.05", "0.1", Some("0.05")),
            (100_000.0, "100000", "100000.0", Some("100000")),
            (1e6, "1e+06", "1000000.0", Some("1000000")),
            (
                123_456_789.0,
                "1.23456789e+08",
                "123456789.0",
                Some("123456789"),
            ),
            (0.0001, "0.0001", "0.0", Some("0.0001")),
            (0.00001, "1e-05", "0.0", Some("0.00001")),
            (1e-7, "1e-07", "0.0", Some("1e-7")),
            (1.5e-7, "1.5e-07", "0.0", Some("1.5e-7")),
            (
                1e20,
                "1e+20",
                "100000000000000000000.0",
                Some("100000000000000000000"),
            ),
            (1e21, "1e+21", "1000000000000000000000.0", Some("1e+21")),
            (5e-324, "5e-324", "0.0", Some("5e-324")),
            (2.675, "2.675", "2.7", Some("2.675")),
            (
                33.333333333333336,
                "33.333333333333336",
                "33.3",
                Some("33.333333333333336"),
            ),
            (f64::NAN, "NaN", "NaN", None),
            (f64::INFINITY, "+Inf", "+Inf", None),
            (f64::NEG_INFINITY, "-Inf", "-Inf", None),
        ] {
            assert_eq!(format_float_go(value), v, "{value:e}");
            assert_eq!(format_fixed_go(value, 1), fixed, "{value:e}");
            assert_eq!(json_float_go(value).as_deref(), json, "{value:e}");
        }
    }
    /// Go 1.26.3 `internal/strconv/pow10tab.go` (generated by `pow10gen.go`):
    /// sha256 over every row as big-endian hi then lo, plus spot rows.
    #[test]
    fn generated_power_of_ten_table_equals_go_pow10tab() {
        use sha2::{Digest, Sha256};
        let table = go_shortest::table();
        assert_eq!(table.len(), 696);
        let mut hasher = Sha256::new();
        for row in table {
            hasher.update(row.to_be_bytes());
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            digest,
            "6fe5dd4274439ffa8a348bfe6ca7c44dda88bef40ac13cf040d19dcd584cc54a"
        );
        for (exponent, row) in [
            (-348, 0xfa8f_d5a0_081c_0288_1732_c869_cd60_e453_u128),
            (-1, 0xcccc_cccc_cccc_cccc_cccc_cccc_cccc_cccc),
            (0, 0x8000_0000_0000_0000_0000_0000_0000_0000),
            (28, 0x813f_3978_f894_0984_4000_0000_0000_0000),
            (56, 0x8281_8f12_81ed_449f_bff8_f10e_7a89_21a4),
            (347, 0xd13e_b464_6944_7567_4b71_95f2_d2d1_a9fb),
        ] {
            assert_eq!(table[(exponent + 348) as usize], row, "1e{exponent}");
        }
    }

    /// Shortest-digit ties are broken to an even last digit as Go does
    /// (captured from Go 1.26.3; Rust's `{:e}` rounds these up).
    #[test]
    fn shortest_digit_ties_round_half_to_even_like_go() {
        for (bits, v) in [
            (0x3ff3_4308_0000_0000_u64, "1.2038650512695312"),
            (0x4057_1292_8000_0000, "92.29019165039062"),
            (0x3e60_0000_0000_0000, "2.9802322387695312e-08"),
        ] {
            assert_eq!(format_float_go(f64::from_bits(bits)), v);
        }
        assert_eq!(
            json_float_go(f64::from_bits(0x3e60_0000_0000_0000)).as_deref(),
            Some("2.9802322387695312e-8")
        );
    }

    #[test]
    fn float32_payload_does_not_expand_to_float64_noise() {
        assert_eq!(float32_json_value(0.2), serde_json::json!(0.2));
        assert_eq!(float32_json_value(0.8), serde_json::json!(0.8));
        assert_eq!(float32_json_value(f32::NAN), serde_json::Value::Null);
    }
}
