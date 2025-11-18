## ✨

- This release attempts to fix one of the bugs associated with some type changes in Rust v1.91, which fails to build the `plist` crate. defaults-rs depends on this crate for the import and export functionalities. This fix downgrades the version used from `v1.8.0` to `v1.7.4` which seems to work, for now.
