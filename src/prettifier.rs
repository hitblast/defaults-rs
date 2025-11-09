// SPDX-License-Identifier: MIT

use crate::PrefValue;

/// Prettify a `PlistValue` in Apple-style format (for CLI).
pub(crate) fn apple_style_string(val: &PrefValue, indent: usize) -> String {
    let ind = |n| "    ".repeat(n);
    match val {
        PrefValue::Dictionary(dict) => {
            let mut out = String::new();
            out.push_str("{\n");
            let iter = dict.iter().peekable();
            for (k, v) in iter {
                out.push_str(&format!(
                    "{}{} = {}",
                    ind(indent + 1),
                    quote_key(k),
                    apple_style_string(v, indent + 1)
                ));
                out.push(';');
                out.push('\n');
            }
            out.push_str(&format!("{}}}", ind(indent)));
            out
        }
        PrefValue::Array(arr) => {
            let mut out = String::new();
            out.push_str("(\n");
            let iter = arr.iter().peekable();
            for v in iter {
                out.push_str(&ind(indent + 1));
                out.push_str(&apple_style_string(v, indent + 1));
                out.push(',');
                out.push('\n');
            }
            out.push_str(&format!("{})", ind(indent)));
            out
        }
        PrefValue::String(s) => quote_string(s),
        PrefValue::Integer(i) => i.to_string(),
        PrefValue::Float(f) => f.to_string(),
        PrefValue::Boolean(b) => {
            if *b {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        PrefValue::Data(data) => format!("<Data: {} bytes>", data.len()),
        PrefValue::Date(dt) => format!("<Date: {}>", dt),
        PrefValue::Url(url) => format!("<Url: {}>", url),
        PrefValue::Uuid(uuid) => format!("<Uuid: {}>", uuid),
    }
}

/// Quotes a key for Apple-style output.
fn quote_key(key: &str) -> String {
    if key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        key.to_string()
    } else {
        format!("\"{}\"", key.replace('"', "\\\""))
    }
}

/// Quotes a string for Apple-style output.
fn quote_string(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('"', "\\\""))
    }
}
