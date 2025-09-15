/// Pretty-printing utilities for Apple-style output.
pub struct Prettifier;

impl Prettifier {
    /// Pretty-print a `PlistValue` in Apple-style format (for CLI).
    pub fn print_apple_style(val: &plist::Value, indent: usize) {
        let ind = |n| "    ".repeat(n);
        match val {
            plist::Value::Dictionary(dict) => {
                println!("{{");
                for (k, v) in dict {
                    print!("{}{} = ", ind(indent + 1), Self::quote_key(k));
                    Self::print_apple_style(v, indent + 1);
                    println!(";");
                }
                print!("{}}}", ind(indent));
            }
            plist::Value::Array(arr) => {
                println!("(");
                for v in arr {
                    print!("{}", ind(indent + 1));
                    Self::print_apple_style(v, indent + 1);
                    println!(",");
                }
                print!("{})", ind(indent));
            }
            plist::Value::String(s) => print!("{}", Self::quote_string(s)),
            plist::Value::Integer(i) => print!("{i}"),
            plist::Value::Real(f) => print!("{f}"),
            plist::Value::Boolean(b) => print!("{}", if *b { "1" } else { "0" }),
            _ => print!("{val:?}"),
        }
    }

    /// Quotes a key for Apple-style output.
    pub fn quote_key(key: &str) -> String {
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
    pub fn quote_string(s: &str) -> String {
        if s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\\\""))
        }
    }
}
