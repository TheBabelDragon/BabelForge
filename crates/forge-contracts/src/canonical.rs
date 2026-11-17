//! Canonical encoding for stable cryptographic hashing.
//!
//! We never hash arbitrary `serde_json::to_string` output.
//! Instead we produce a deterministic key-sorted, whitespace-free
//! representation suitable for content-addressing.

use sha2::{Digest, Sha256};
use serde_json::Value;

/// Produce a canonical JSON string (sorted keys, no whitespace).
pub fn canonical_json(value: &Value) -> Result<String, String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if c.is_control() => {
                        out.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
            Ok(out)
        }
        Value::Array(arr) => {
            let parts: Result<Vec<_>, _> = arr.iter().map(canonical_json).collect();
            Ok(format!("[{}]", parts?.join(",")))
        }
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut parts = Vec::with_capacity(keys.len());
            for k in keys {
                let v = map.get(&k).unwrap();
                let ck = canonical_json(&Value::String(k))?;
                let cv = canonical_json(v)?;
                parts.push(format!("{}:{}", ck, cv));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
    }
}

/// SHA-256 hex digest of the canonical encoding.
pub fn canonical_hash(value: &Value) -> Result<String, String> {
    let canon = canonical_json(value)?;
    let mut hasher = Sha256::new();
    hasher.update(canon.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_keys_are_sorted() {
        let a = json!({"b": 1, "a": 2});
        let b = json!({"a": 2, "b": 1});
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
        assert_eq!(canonical_hash(&a).unwrap(), canonical_hash(&b).unwrap());
    }

    #[test]
    fn nested_stable() {
        let v = json!({"z": {"y": [3, 1, 2], "x": true}, "a": null});
        let h = canonical_hash(&v).unwrap();
        assert_eq!(h.len(), 64);
    }
}
