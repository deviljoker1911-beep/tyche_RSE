//! Canonical JSON serialization.
//!
//! Tyche commits to portfolio inputs and simulation outputs by hashing their
//! JSON representation. Hashes are reproducible only if the JSON encoding is
//! itself canonical: object keys sorted lexicographically, no whitespace, no
//! locale-dependent number formatting.
//!
//! This module provides a single function, [`canonical_json`], that produces
//! such an encoding for any `serde::Serialize` value.
//!
//! # Why not `serde_json::to_vec`?
//!
//! `serde_json` preserves insertion order of map keys and has implementation-
//! defined whitespace settings (none today, but not contractually). The
//! canonical encoder here:
//!
//! - sorts object keys by their UTF-8 byte order;
//! - emits no whitespace;
//! - emits floats via Rust's `f64` `to_string` (round-trip lossless via Grisu);
//! - rejects `NaN` and infinities, which have no JSON representation.

use std::io::Write;

use serde::Serialize;
use serde_json::Value;

use crate::TypesError;

/// Serialize a value to canonical JSON bytes.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, TypesError> {
    let v = serde_json::to_value(value)?;
    let mut out = Vec::with_capacity(256);
    write_canonical(&mut out, &v)?;
    Ok(out)
}

fn write_canonical(buf: &mut Vec<u8>, value: &Value) -> Result<(), TypesError> {
    match value {
        Value::Null => {
            buf.extend_from_slice(b"null");
        }
        Value::Bool(b) => {
            buf.extend_from_slice(if *b { b"true" } else { b"false" });
        }
        Value::Number(n) => {
            // Integer fast-path keeps "1" rather than "1.0".
            //
            // `write!` to a `Vec<u8>` returns `io::Result`, but `Vec`'s
            // `io::Write` impl is infallible — it only fails on OOM, which
            // panics elsewhere first. Using `.expect()` documents the
            // invariant while keeping the surface clean.
            const VEC_WRITE_INFALLIBLE: &str = "write to Vec<u8> is infallible";
            if let Some(i) = n.as_i64() {
                write!(buf, "{i}").expect(VEC_WRITE_INFALLIBLE);
            } else if let Some(u) = n.as_u64() {
                write!(buf, "{u}").expect(VEC_WRITE_INFALLIBLE);
            } else if let Some(f) = n.as_f64() {
                if !f.is_finite() {
                    return Err(TypesError::Invariant(format!(
                        "non-finite float in canonical JSON: {f}"
                    )));
                }
                // Rust's default `{}` for f64 round-trips losslessly via Grisu.
                // Match JSON conventions: integer-valued floats render with no
                // decimal point so {"x": 1.0} canonicalises identically to
                // {"x": 1}.
                if f.fract() == 0.0 && f.abs() < 1e16 {
                    write!(buf, "{}", f as i64).expect(VEC_WRITE_INFALLIBLE);
                } else {
                    write!(buf, "{f}").expect(VEC_WRITE_INFALLIBLE);
                }
            } else {
                return Err(TypesError::Invariant("unsupported number".into()));
            }
        }
        Value::String(s) => write_string(buf, s),
        Value::Array(items) => {
            buf.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical(buf, item)?;
            }
            buf.push(b']');
        }
        Value::Object(map) => {
            // Sort keys lexicographically by UTF-8 bytes.
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            buf.push(b'{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_string(buf, k);
                buf.push(b':');
                write_canonical(buf, v)?;
            }
            buf.push(b'}');
        }
    }
    Ok(())
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    buf.push(b'"');
    for c in s.chars() {
        match c {
            '"' => buf.extend_from_slice(b"\\\""),
            '\\' => buf.extend_from_slice(b"\\\\"),
            '\n' => buf.extend_from_slice(b"\\n"),
            '\r' => buf.extend_from_slice(b"\\r"),
            '\t' => buf.extend_from_slice(b"\\t"),
            '\u{08}' => buf.extend_from_slice(b"\\b"),
            '\u{0c}' => buf.extend_from_slice(b"\\f"),
            c if (c as u32) < 0x20 => {
                // `write!` to `Vec<u8>` cannot fail; see `write_canonical` for
                // the full rationale.
                write!(buf, "\\u{:04x}", c as u32).expect("write to Vec<u8> is infallible");
            }
            c => {
                let mut tmp = [0u8; 4];
                buf.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
            }
        }
    }
    buf.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn key_order_is_canonical() {
        let a = json!({"b": 1, "a": 2, "c": 3});
        let b = json!({"a": 2, "b": 1, "c": 3});
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
        assert_eq!(
            String::from_utf8(canonical_json(&a).unwrap()).unwrap(),
            r#"{"a":2,"b":1,"c":3}"#
        );
    }

    #[test]
    fn integer_floats_render_as_integers() {
        let v = json!({"x": 1.0});
        assert_eq!(
            String::from_utf8(canonical_json(&v).unwrap()).unwrap(),
            r#"{"x":1}"#
        );
    }

    #[test]
    fn nested_objects_are_canonical() {
        let a = json!({"outer": {"z": 1, "a": 2}, "inner": [3, 2, 1]});
        let b = json!({"inner": [3, 2, 1], "outer": {"a": 2, "z": 1}});
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
    }

    #[test]
    fn serde_json_already_rejects_nan() {
        // serde_json's `Number::from_f64` returns `None` for non-finite values.
        // Our canonical encoder relies on this upstream guarantee so we
        // codify it as a regression test rather than reach Value construction.
        assert!(serde_json::Number::from_f64(f64::NAN).is_none());
        assert!(serde_json::Number::from_f64(f64::INFINITY).is_none());
        assert!(serde_json::Number::from_f64(f64::NEG_INFINITY).is_none());
    }

    #[test]
    fn string_escaping() {
        let v = json!("a\"b\\c\nd");
        assert_eq!(
            String::from_utf8(canonical_json(&v).unwrap()).unwrap(),
            r#""a\"b\\c\nd""#
        );
    }
}
