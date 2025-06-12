/// Types for preferences domains and values.
use std::collections::HashMap;

/// Represents a preferences domain (user or global).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Domain {
    /// A user domain, e.g., "com.apple.finder"
    User(String),
    /// The global preferences domain (".GlobalPreferences")
    Global,
}

/// Represents a value stored in preferences.
/// This is a simplified version; expand as needed.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<PrefValue>),
    Dictionary(HashMap<String, PrefValue>),
    // Add more types as needed
}
