<div align="center">

<img src="assets/logo.png">

# <img src="https://raw.githubusercontent.com/github/explore/80688e429a7d4ef2fca1e82350fe8e3517d3494d/topics/rust/rust.png" width="40px"> defaults-rs

Near drop-in replacement for the macOS `defaults` CLI with API bindings for Rust

</div>

> [!IMPORTANT]
> Consider starring the project if you like it! It really supports and motivates me to make more projects like these.

---

## Table of Contents

- [Key Features](#key-features)
- [Installation](#installation)
- [CLI Usage](#cli-usage)
- [Rust API](#rust-api)
- [License](#license)

---

## Key Features

- **CLI (`drs`)**: Use it as a direct replacement for `defaults` without hassle.
- **Async Rust API**: Read, write, delete, rename, import/export, and inspect preferences from Rust code.
- **Supports**: User and global domains, all plist value types (int, float, bool, string, arrays, dictionaries).
- **Familiar Style**: Pretty-prints plist data close to the original `defaults` format.
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

> [!NOTE]
> `defaults-rs` requires Rust **(v1.70 or greater)** to be installed on your machine. Also, builds are only possible on macOS.

```sh
cargo install --git https://github.com/hitblast/defaults-rs
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
drs write com.apple.dock tilesize -i 48
# or
drs write com.apple.dock tilesize --int 48
```

#### Delete a key

```sh
drs delete com.apple.dock tilesize
```

#### Read the whole domain

```sh
drs read com.apple.dock
```

#### List all entries in all domains containing word

```sh
drs find <word>
```

#### View all domains

```sh
drs domains
```

#### Use the global domain

```sh
drs read -g com.apple.keyboard.fnState
drs write -g InitialKeyRepeat --int 25
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

Please refer to the [API reference](https://hitblast.github.io/defaults-rs) for more information about all the available functions.

### Example

```rust
use defaults_rs::{Domain, PrefValue, Preferences};

#[tokio::main]
async fn main() {
    // Read a value
    let value = Preferences::read(Domain::User("com.apple.dock".into()), Some("tilesize"))
        .await
        .unwrap();

    // Write a value
    Preferences::write(
        Domain::User("com.apple.dock".into()),
        "tilesize",
        PrefValue::Integer(48),
    )
    .await
    .unwrap();

    // Delete a key
    Preferences::delete(Domain::User("com.apple.dock".into()), Some("tilesize"))
        .await
        .unwrap();
}
```

For writing domains in batches, you can use the batch-write function:

```rust
use defaults_rs::{Domain, PrefValue, Preferences};

#[tokio::main]
async fn main() {
    let batch = vec![
        (
            Domain::User("com.apple.dock".into()),
            vec![
                ("tilesize".into(), PrefValue::Integer(48)),
                ("autohide".into(), PrefValue::Boolean(true)),
            ],
        ),
        (
            Domain::User("com.apple.keyboard".into()),
            vec![
                ("InitialKeyRepeat".into(), PrefValue::Integer(25)),
            ],
        ),
    ];
    Preferences::write_batch(batch).await.unwrap();
}
```

---

## License

This project has been licensed under the [MIT License](./LICENSE).
