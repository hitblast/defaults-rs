//! Library API for defaults-rs: macOS preferences management in Rust.

mod prefs;

pub use prefs::error::PrefError;
pub use prefs::types::{Domain, PrefValue, ReadResult};

pub mod preferences;
pub use preferences::Preferences;

pub mod prettifier;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use cli::build_cli;
