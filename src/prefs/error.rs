//! Error types for preferences management.

use std::fmt;

/// Errors that can occur when interacting with preferences.
#[derive(Debug)]
pub enum PrefError {
    Io(std::io::Error),
    // Plist(plist::Error), // Uncomment when plist crate is added
    KeyNotFound,
    InvalidType,
    Other(String),
}

impl fmt::Display for PrefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrefError::Io(e) => write!(f, "IO error: {}", e),
            // PrefError::Plist(e) => write!(f, "Plist error: {}", e),
            PrefError::KeyNotFound => write!(f, "Key not found"),
            PrefError::InvalidType => write!(f, "Invalid type"),
            PrefError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for PrefError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PrefError::Io(e) => Some(e),
            // PrefError::Plist(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PrefError {
    fn from(e: std::io::Error) -> Self {
        PrefError::Io(e)
    }
}

// Uncomment when plist crate is added
// impl From<plist::Error> for PrefError {
//     fn from(e: plist::Error) -> Self {
//         PrefError::Plist(e)
//     }
// }
