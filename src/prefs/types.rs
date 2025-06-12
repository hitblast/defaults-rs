use std::collections::HashMap;

/// Preferences domain (user or global).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Domain {
    /// A user domain, e.g., "com.apple.finder"
    User(String),
    /// The global preferences domain (".GlobalPreferences")
    Global,
}

/// Value stored in preferences.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<PrefValue>),
    Dictionary(HashMap<String, PrefValue>),
}

// Extension: Add helper on PrefValue
impl PrefValue {
    pub fn from_str(type_flag: &str, s: &str) -> Result<Self, String> {
        match type_flag {
            "int" => s
                .parse::<i64>()
                .map(PrefValue::Integer)
                .map_err(|_| "Invalid integer value".into()),
            "float" => s
                .parse::<f64>()
                .map(PrefValue::Float)
                .map_err(|_| "Invalid float value".into()),
            "bool" => match s {
                "true" | "1" => Ok(PrefValue::Boolean(true)),
                "false" | "0" => Ok(PrefValue::Boolean(false)),
                _ => Err("Invalid boolean value (use true/false or 1/0)".into()),
            },
            "string" => Ok(PrefValue::String(s.to_string())),
            other => Err(format!("Unsupported type: {other}")),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            PrefValue::String(_) => "string",
            PrefValue::Integer(_) => "integer",
            PrefValue::Float(_) => "float",
            PrefValue::Boolean(_) => "boolean",
            PrefValue::Array(_) => "array",
            PrefValue::Dictionary(_) => "dictionary",
        }
    }
}

/// Result of a read operation: either a single value or a whole plist.
pub enum ReadResult {
    Value(PrefValue),
    Plist(plist::Value),
}
