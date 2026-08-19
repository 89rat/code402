//! Minimal RFC 8785 (JCS) canonical JSON serializer for the receipt I/O
//! domain. XDR-1 v0.2 pins JCS so `input_hash`/`output_hash` are reproducible
//! by third parties with any conforming encoder.
//!
//! Scope (fail-closed): objects (keys sorted by UTF-16 code units per
//! §3.2.3), arrays, strings (§3.2.2.2 escaping), bool/null, and INTEGERS.
//! Floats are REJECTED — RFC 8785 float serialization is ES6-number
//! formatting; rather than approximate it, we refuse values outside the
//! receipt domain (validated tool I/O never emits floats).

use alloy_primitives::{keccak256, B256};

/// UTF-16 code-unit ordering (RFC 8785 §3.2.3): compare keys by their
/// UTF-16 encoding, element-wise, shorter prefix first.
fn utf16_less(a: &str, b: &str) -> std::cmp::Ordering {
    let au: Vec<u16> = a.encode_utf16().collect();
    let bu: Vec<u16> = b.encode_utf16().collect();
    au.cmp(&bu)
}

fn write_escaped(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{0008}' => out.extend_from_slice(b"\\b"),
            '\u{000c}' => out.extend_from_slice(b"\\f"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

fn write_value(v: &serde_json::Value, out: &mut Vec<u8>) -> Result<(), String> {
    match v {
        serde_json::Value::Null => out.extend_from_slice(b"null"),
        serde_json::Value::Bool(true) => out.extend_from_slice(b"true"),
        serde_json::Value::Bool(false) => out.extend_from_slice(b"false"),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.extend_from_slice(i.to_string().as_bytes());
            } else if let Some(u) = n.as_u64() {
                out.extend_from_slice(u.to_string().as_bytes());
            } else {
                return Err(format!("float in canonicalization domain: {n}"));
            }
        }
        serde_json::Value::String(s) => write_escaped(s, out),
        serde_json::Value::Array(a) => {
            out.push(b'[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(item, out)?;
            }
            out.push(b']');
        }
        serde_json::Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort_by(|a, b| utf16_less(a, b));
            out.push(b'{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_escaped(k, out);
                out.push(b':');
                write_value(&m[k.as_str()], out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// RFC 8785 canonical bytes; Err on anything outside the integer/string
/// JSON domain (fail closed — never approximate a canonical form).
pub fn jcs_bytes(v: &serde_json::Value) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    write_value(v, &mut out)?;
    Ok(out)
}

/// keccak256(jcs(v)) — the XDR-1 I/O hash rule.
pub fn jcs_hash(v: &serde_json::Value) -> Result<B256, String> {
    Ok(keccak256(jcs_bytes(v)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_sort_by_utf16_code_units() {
        let a = json!({"b": 1, "a": 2});
        let b = json!({"a": 2, "b": 1});
        assert_eq!(jcs_bytes(&a).unwrap(), jcs_bytes(&b).unwrap());
        assert_eq!(jcs_bytes(&a).unwrap(), br#"{"a":2,"b":1}"#);
        // UTF-16 order: U+FF21 (fullwidth A) sorts AFTER ascii 'z'
        let u = json!({ "\u{FF21}": 1, "z": 2 });
        let c = String::from_utf8(jcs_bytes(&u).unwrap()).unwrap();
        let expected = String::from("{\"z\":2,\"\u{FF21}\":1}");
        assert_eq!(c, expected);
    }

    #[test]
    fn string_escaping_and_structure() {
        let v = json!({"s": "a\"b\\c\nd\u{0001}", "arr": [1, {"y": true, "x": null}]});
        let c = String::from_utf8(jcs_bytes(&v).unwrap()).unwrap();
        // control char escaped as \u0001; keys sorted; no whitespace
        let expected = String::from("{\"arr\":[1,{\"x\":null,\"y\":true}],\"s\":\"a\\\"b\\\\c\\nd\\u0001\"}");
        assert_eq!(c, expected);
    }

    #[test]
    fn floats_fail_closed() {
        assert!(jcs_bytes(&json!({"x": 1.5})).is_err());
        assert!(jcs_bytes(&json!([1.0])).is_err());
    }

    #[test]
    fn jcs_hash_is_stable_regardless_of_insertion_order() {
        let a = jcs_hash(&json!({"vat_number": "GB123456789", "n": 3})).unwrap();
        let b = jcs_hash(&json!({"n": 3, "vat_number": "GB123456789"})).unwrap();
        assert_eq!(a, b);
    }
}
