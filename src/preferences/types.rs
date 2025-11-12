// SPDX-License-Identifier: MIT

use crate::PrefValue;

/// Preferences domain (user or global).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Domain {
    /// A user domain, e.g., "com.apple.finder"
    User(String),
    /// The global preferences domain / NSGlobalDomain (".GlobalPreferences")
    Global,
}

impl Domain {
    /// Returns the CoreFoundation name for a given domain.
    pub fn get_cf_name(&self) -> String {
        match &self {
            Domain::Global => String::from(".GlobalPreferences"),
            Domain::User(name) => name.clone(),
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::User(s) => write!(f, "{}", s),
            Domain::Global => write!(f, "NSGlobalDomain"),
        }
    }
}

/// Result of a find operation.
#[derive(Debug)]
pub struct FindMatch {
    pub key: String,
    pub value: PrefValue,
}
