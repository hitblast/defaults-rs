<img src="assets/logo.png" width="170px" align="right">

# 🍎 defaults-rs

#### `defaults` replacement for macOS

![Crates.io Total Downloads](https://img.shields.io/crates/d/defaults-rs)
[![Refactor CI](https://github.com/hitblast/defaults-rs/actions/workflows/tests.yml/badge.svg)](https://github.com/hitblast/defaults-rs/actions/workflows/tests.yml)

<img src="assets/demo.gif">

## Key Features

- Read, write, delete, rename, import/export, and inspect preferences.
- Supports user/global/path domains.
- Supports *all* plist value types.
- Pretty-printing and better logging than the original.
- Dynamically chooses between XML and binary PLIST data formats.

> [!WARNING]
> Some specs might not be applicable (e.g. adding to dict) currently for the CLI side, so the API side is exposed fully for proper extensibility.

## Installation

### Using `brew`:

```sh
$ brew install hitblast/tap/drs
```

### Using `cargo`:

```sh
$ cargo install defaults-rs
```

### Using `mise`:

```sh
# NOTE: This will compile the binary for your system.
$ mise use -g cargo:defaults-rs
```

## CLI Usage

The CLI command is `drs`. It closely mimics the original `defaults` tool.

### Examples

#### Read a key (domain or path)

```sh
$ drs read com.apple.dock tilesize
$ drs read ~/Library/Preferences/com.apple.dock.plist tilesize
$ drs read ./custom.plist mykey
$ drs read com.apple.dock.plist tilesize   # if file exists, treated as path; else as domain
```

#### Write a key

```sh
$ drs write com.apple.dock tilesize -i 48
$ drs write com.apple.dock tilesize --int 48
$ drs write ~/Library/Preferences/com.apple.dock.plist tilesize --int 48
$ drs write ./custom.plist mykey --string "hello"
```

#### Delete a key

```sh
$ drs delete com.apple.dock tilesize
$ drs delete ~/Library/Preferences/com.apple.dock.plist tilesize
$ drs delete ./custom.plist mykey
```

#### Read the whole domain

```sh
$ drs read com.apple.dock
$ drs read ~/Library/Preferences/com.apple.dock.plist
$ drs read ./custom.plist
```

#### List all entries in all domains containing word

```sh
$ drs find <word>
```

#### View all domains

```sh
$ drs domains
```

#### Use the global domain

```sh
$ drs read -g com.apple.keyboard.fnState
$ drs write -g InitialKeyRepeat --int 25
```

#### Read the type of a key

```sh
$ drs read-type com.apple.dock tilesize
```

#### Rename a key

```sh
$ drs rename com.apple.dock oldKey newKey
$ drs rename ~/Library/Preferences/com.apple.dock.plist oldKey newKey
$ drs rename ./custom.plist oldKey newKey
```

#### Import/export a domain

```sh
$ drs import com.apple.dock ./mysettings.plist
$ drs export com.apple.dock ./backup.plist
```

## Rust API

To access the developer-side API for `defaults-rs`, run the following command and add it to your Cargo project:

```sh
$ cargo add defaults-rs --no-default-features
```

Please refer to the [API reference](https://hitblast.github.io/defaults-rs) for more information about all the available functions.

### Example

```rust
use defaults_rs::{Domain, PrefValue, Preferences};

#[tokio::main]  // `cargo add tokio`
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

The API also provides unified batch functions which can significantly reduce the amount of I/O per read/write/delete if you want to do multiple queries.

```rust
use anyhow::Result;  // `cargo add anyhow`
use defaults_rs::{Domain, PrefValue, Preferences};

#[tokio::main]  // `cargo add tokio`
async fn main() -> Result<()> {
    // Batch write (only updates designated keys)
    let write_batch = vec![
        (
            Domain::User("com.apple.dock".into()),
            "tilesize".into(),
            PrefValue::Integer(48),
        ),
        (
            Domain::User("com.apple.dock".into()),
            "autohide".into(),
            PrefValue::Boolean(true),
        ),
        (
            Domain::User("com.apple.keyboard".into()),
            "InitialKeyRepeat".into(),
            PrefValue::Integer(25),
        ),
    ];
    Preferences::write_batch(write_batch).await?;

    // Batch read:
    let read_batch = vec![
        (
            Domain::User("com.apple.dock".into()),
            Some("tilesize".into()),
        ),
        (Domain::User("com.apple.keyboard".into()), None), // Read entire domain
    ];
    let results = Preferences::read_batch(read_batch).await?;
    for (domain, key, result) in results {
        match key {
            None => println!("Domain: {:?}, Full plist: {:?}", domain, result),
            Some(k) => println!("Domain: {:?}, Key: {:?}, Value: {:?}", domain, k, result),
        }
    }

    // Batch delete:
    let delete_batch = vec![
        (
            Domain::User("com.apple.dock".into()),
            Some("tilesize".into()),
        ),
        (
            Domain::User("com.apple.dock".into()),
            Some("autohide".into()),
        ),
        (Domain::User("com.apple.keyboard".into()), None), // Delete entire domain file
    ];
    Preferences::delete_batch(delete_batch).await?;

    Ok(())
}
```

## Contributing

New pull requests and issues are always welcome. Please read the [contribution guidelines](./CONTRIBUTING.md) for more information about certain parts of the codebase and/or how to form a structured pull request.

## License

This project has been licensed under the [MIT License](./LICENSE).
