//! Library API for defaults-rs: macOS preferences management in Rust.

mod prefs;

pub use prefs::error::PrefError;
pub use prefs::types::{Domain, PrefValue, ReadResult};

pub mod preferences;
pub mod prettifier;

pub mod cli;
pub use cli::build_cli;
