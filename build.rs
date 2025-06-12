fn main() {
    #[cfg(not(target_os = "macos"))]
    panic!("`defaults-rs` only works on macOS and darwin-based platforms.");
}
