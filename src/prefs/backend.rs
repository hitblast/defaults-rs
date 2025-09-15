//! Backend selection logic for defaults-rs preferences API.
//!
//! This module is responsible for:
//! - Determining which backend (CoreFoundation or file-based) should be used for preferences operations.
//! - Exposing the `ActiveBackend` enum and the `BACKEND` static instance for use in the preferences API.
//!
//! No business logic or CLI parsing is performed here.
//! All backend selection is separated from preferences management and user interaction.

use once_cell::sync::Lazy;

/// Backend selection for preferences (CoreFoundation vs File).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveBackend {
    CoreFoundation,
    File,
}

/// Static instance that determines the active backend at runtime.
pub static BACKEND: Lazy<ActiveBackend> = Lazy::new(|| {
    // Import the cf_available function from the core backend.
    use crate::prefs::core;
    if core::cf_available() {
        ActiveBackend::CoreFoundation
    } else {
        ActiveBackend::File
    }
});
