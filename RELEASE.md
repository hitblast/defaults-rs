## ✨

- `Preferences::find` now returns a hashmap of `Domain` as keys instead of a binary tree map with `String` keys for easier `Domain` lookup.
- `clippy` lints have been made tighter to avoid unexpected panics in code, effectively making them zero in the codebase alone.
- Enhanced graceful error handling across the core.
