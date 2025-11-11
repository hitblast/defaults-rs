// SPDX-License-Identifier: MIT

//! Library API for defaults-rs: macOS preferences management in Rust.

mod core;
pub use core::types::PrefValue;

mod preferences;
pub use preferences::Preferences;
pub use preferences::types::{Domain, FindMatch};

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use cli::build_cli;
