// SPDX-License-Identifier: MIT

use crate::PrefValue;

/// Prettify a `PlistValue` for display.
///
/// This essentially takes all complex types such as PrefValue::Dictionary or PrefValue::Array, and turns
/// them into indented syntactic sugar output for the terminal.
pub(crate) fn prettify(val: &PrefValue, indent: usize) -> String {
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
                    prettify(v, indent + 1)
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
                out.push_str(&prettify(v, indent + 1));
                out.push(',');
                out.push('\n');
            }
            out.push_str(&format!("{})", ind(indent)));
            out
        }
        PrefValue::String(s) => quote_string(s),
        PrefValue::Boolean(b) => {
            if *b {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        PrefValue::Data(data) => {
            let mut s = data
                .iter()
                .map(|b| format!("0x{:02X}", b))
                .collect::<Vec<_>>();
            s.truncate(5);

            format!("<length = {}, bytes = [{}...]>", data.len(), s.join(", "))
        }
        PrefValue::Url(url) => format!("<URL: {}>", url),
        PrefValue::Uuid(uuid) => format!("<UUID: {}>", uuid),
        PrefValue::Uid(uid) => format!("<UID: {}>", uid),
        _ => val.to_string(),
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
