//! Go `encoding/json` byte layout for model artifacts and operator views.
//!
//! `serde_json` and Go agree on structure but not on number and string text:
//! Go writes the float `1` as `1` (serde: `1.0`), switches to exponent form
//! below `1e-6` and from `1e21` with a signed two-digit-trimmed exponent
//! (`1e-7`, `1e+21`), and escapes `<`, `>`, `&`, U+2028 and U+2029. This
//! formatter reproduces those rules so a Rust-written bundle or snapshot is
//! byte-identical to the legacy Go output for the same values; the digest of a
//! bundle then identifies the same bytes in both implementations.
//!
//! Field order is still the serializer's (struct declaration order); callers
//! keep Go's struct order, and Go's sorted order for map-valued documents.
//! Non-finite floats never reach this formatter (`serde_json` writes `null`
//! where Go returns an error), so callers must only encode finite values.
use serde::Serialize;
use serde_json::ser::{CompactFormatter, Formatter, PrettyFormatter};
use std::io;

/// Shortest round-trip text of a finite float in Go's `encoding/json` style.
/// `positional` is Rust's shortest positional `Display` text, `scientific` its
/// shortest `{:e}` text, `exponent_form` whether Go uses the `e` format.
fn go_float(positional: String, scientific: String, exponent_form: bool) -> String {
    if !exponent_form {
        return positional;
    }
    // Rust `{:e}` is `d[.ddd]e[-]x`; Go is `d[.ddd]e(+|-)x` with at least one digit.
    match scientific.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => scientific,
    }
}

/// Go float32 text (`strconv.AppendFloat(.., 'f'|'e', -1, 32)` plus cleanup).
pub fn go_f32(value: f32) -> String {
    let abs = value.abs();
    go_float(
        format!("{value}"),
        format!("{value:e}"),
        abs != 0.0 && !(1e-6..1e21).contains(&abs),
    )
}

/// Go float64 text (`strconv.AppendFloat(.., 'f'|'e', -1, 64)` plus cleanup).
pub fn go_f64(value: f64) -> String {
    let abs = value.abs();
    go_float(
        format!("{value}"),
        format!("{value:e}"),
        abs != 0.0 && !(1e-6..1e21).contains(&abs),
    )
}

/// Wraps a structural formatter and overrides number and string text.
struct GoFormatter<F>(F);

impl<F: Formatter> Formatter for GoFormatter<F> {
    fn write_f32<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f32) -> io::Result<()> {
        writer.write_all(go_f32(value).as_bytes())
    }

    fn write_f64<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        writer.write_all(go_f64(value).as_bytes())
    }

    fn write_string_fragment<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        let mut start = 0;
        for (index, character) in fragment.char_indices() {
            let escape = match character {
                '<' => "\\u003c",
                '>' => "\\u003e",
                '&' => "\\u0026",
                '\u{2028}' => "\\u2028",
                '\u{2029}' => "\\u2029",
                _ => continue,
            };
            writer.write_all(&fragment.as_bytes()[start..index])?;
            writer.write_all(escape.as_bytes())?;
            start = index + character.len_utf8();
        }
        writer.write_all(&fragment.as_bytes()[start..])
    }

    fn begin_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_array(writer)
    }

    fn end_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_array(writer)
    }

    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.0.begin_array_value(writer, first)
    }

    fn end_array_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_array_value(writer)
    }

    fn begin_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_object(writer)
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_object(writer)
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.0.begin_object_key(writer, first)
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_object_value(writer)
    }

    fn end_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_object_value(writer)
    }
}

fn encode<T: Serialize + ?Sized, F: Formatter>(
    value: &T,
    formatter: F,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut out = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, GoFormatter(formatter));
    value.serialize(&mut serializer)?;
    Ok(out)
}

/// Go `json.MarshalIndent(value, "", indent)`.
pub fn to_vec_indent<T: Serialize + ?Sized>(
    value: &T,
    indent: &[u8],
) -> Result<Vec<u8>, serde_json::Error> {
    encode(value, PrettyFormatter::with_indent(indent))
}

/// Go `json.Marshal(value)`; Go's `Encoder.Encode` additionally appends `\n`.
pub fn to_vec<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    encode(value, CompactFormatter)
}

#[cfg(test)]
mod tests;
