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

/// Result of a read operation: either a single value or a whole plist.
pub enum ReadResult {
    Value(PrefValue),
    Plist(plist::Value),
}
