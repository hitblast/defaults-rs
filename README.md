# <img src="https://raw.githubusercontent.com/github/explore/80688e429a7d4ef2fca1e82350fe8e3517d3494d/topics/rust/rust.png" width="40px"> defaults-rs

Near drop-in replacement macOS defaults CLI with a Rust-facing API binding

> [!IMPORTANT]
> Consider starring the project if you like it! It really supports and motivates me to make more projects like these.

---

## Key Features

- **CLI (`drs`)**: Drop-in replacement for the `defaults` command on macOS.
- **Async Rust API**: Read, write, delete, rename, import/export, and inspect preferences from Rust code.
- **Supports**: User and global domains, all plist value types (int, float, bool, string, arrays, dictionaries).
- **Apple-style Output**: Pretty-prints plist data in the familiar macOS format.
- **Binary & XML Plist**: Handles both binary and XML plist files transparently.
- **Extensible**: Designed for easy addition of new commands and value types.

---

## Installation

### Using `brew`:

```sh
brew install hitblast/tap/drs
```

### Using `cargo`:

```sh
cargo install defaults-rs
```

### Manual Build & Install

```sh
git clone https://github.com/hitblast/defaults-rs.git
cd defaults-rs
cargo install --path .
```

---

## CLI Usage

The CLI command is `drs`. It closely mimics the original `defaults` tool.

### Examples

#### Read a key

```sh
drs read com.apple.dock tilesize
```

#### Write a key

```sh
drs write com.apple.dock tilesize --type int 48
```

#### Delete a key

```sh
drs delete com.apple.dock tilesize
```

#### Read the whole domain

```sh
drs read com.apple.dock
```

#### Use the global domain

```sh
drs read -g com.apple.keyboard.fnState
drs write -g InitialKeyRepeat --type int 25
```

#### Read the type of a key

```sh
drs read-type com.apple.dock tilesize
```

#### Rename a key

```sh
drs rename com.apple.dock oldKey newKey
```

#### Import/Export a domain

```sh
drs import com.apple.dock ./mysettings.plist
drs export com.apple.dock ./backup.plist
```

---

## Rust API

To access the developer-side API for `defaults-rs`, run the following command and add it to your Cargo project:

```sh
cargo add defaults-rs
```

### Example

```rust
use defaults_rs::{Preferences, Domain, PrefValue};

#[tokio::main]
async fn main() {
    // Read a value
    let value = Preferences::read(Domain::User("com.apple.dock".into()), Some("tilesize")).await.unwrap();

    // Write a value
    Preferences::write(Domain::User("com.apple.dock".into()), "tilesize", PrefValue::Integer(48)).await.unwrap();

    // Delete a key
    Preferences::delete(Domain::User("com.apple.dock".into()), Some("tilesize")).await.unwrap();
}
```

---

## License

This project has been licensed under the [MIT License](./LICENSE).
