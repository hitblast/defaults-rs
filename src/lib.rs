// SPDX-License-Identifier: MIT

//! Library API for defaults-rs: macOS preferences management in Rust.

mod core;

pub use core::error::PrefError;
pub use core::types::{Domain, PrefValue, ReadResult};

mod preferences;
pub use preferences::Preferences;

pub mod prettifier;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use cli::build_cli;
