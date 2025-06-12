//! Library API for defaults-rs: macOS preferences management in Rust.
//!
//! This crate provides an API for reading, writing, and deleting user and global
//! preferences (plist files) on macOS, similar to the `defaults` CLI.

mod prefs;

pub use prefs::error::PrefError;
pub use prefs::types::{Domain, PrefValue};

/// Main struct for interacting with preferences.
pub mod preferences;
