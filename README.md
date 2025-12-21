# 🍎 defaults-rs

#### Open-source interface to a user's defaults on macOS

![Crates.io Total Downloads](https://img.shields.io/crates/d/defaults-rs)
[![Refactor CI](https://github.com/machlit/defaults-rs/actions/workflows/refactor.yml/badge.svg)](https://github.com/machlit/defaults-rs/actions/workflows/refactor.yml)

<img src="assets/demo.gif">

## Key Features

- Read, write, delete, rename, import/export, and inspect preferences.
- Supports user/global/path domains.
- Supports *all* plist value types (API).
- Pretty-printing and better logging than the original `defaults` tool.
- **Execution safety.** Never accidentally write to a faulty domain again.
- Dynamically chooses between XML and binary PLIST data formats.
- Extremely small (<1.5k SLoC).

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Rust API Usage](#rust-api-usage)
- [Why defaults-rs](#why-defaults-rs)
- [Caveats](#caveats)
- [Contributing](#contributing)
- [License](#license)

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

## Usage

The CLI command is `drs`. It closely mimics the original `defaults` tool.

#### Read a key (domain or path)

```sh
$ drs read com.apple.dock tilesize
$ drs read ~/Library/Preferences/com.apple.dock.plist tilesize
$ drs read com.apple.dock.plist tilesize   # this also works!
```

#### Write a key

```sh
$ drs write com.apple.dock tilesize -i 48
$ drs write com.apple.dock tilesize --int 48
$ drs write ~/Library/Preferences/com.apple.dock.plist tilesize --int 48

# create a new domain (disables checks)
$ drs write rubberduck --force duckcount --int 5
```

#### Delete a key

```sh
$ drs delete com.apple.dock tilesize
$ drs delete ~/Library/Preferences/com.apple.dock.plist tilesize
```

#### Read the whole domain

```sh
$ drs read com.apple.dock
$ drs read ~/Library/Preferences/com.apple.dock.plist
```

#### List all entries in all domains containing word

```sh
$ drs find <word>
```

#### View / fuzzy-read domains

```sh
$ drs domains

# disable fuzzy-searching with the -n/--no-fuzzy flag
$ drs domains -n
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
```

#### Import/export a domain

```sh
$ drs import com.apple.dock ./mysettings.plist
$ drs export com.apple.dock ./backup.plist
```

## Rust API Usage

In order to use the Rust API for defaults-rs, run this command in your project directory:

```sh
$ cargo add defaults-rs --no-default-features
```

### API Reference

Check out the official docs.rs API reference for defaults-rs [here](https://docs.rs/defaults-rs/).

For examples, check out: [examples/](https://github.com/machlit/defaults-rs/tree/master/examples)

## Why defaults-rs

defaults-rs was initially made with the necessity of a thin wrapper around the CoreFoundation APIs which are
responsible for storing a user's defaults. Now, it also serves as a backend for directly interfering with
system preferences in the [cutler](https://github.com/machlit/cutler) project.

## Caveats

Some temporary limitations include:

- `.plist` files for applications (`Info.plist` specifically) cannot be read by this tool as of now, although this is planned for the next release.

And since this is a completely open-source attempt to replicate the capabilities of `defaults` (which is a proprietary software), there will be certain limitations:

- Domain-reading might result in showing slightly "less" output in very rare cases where parts of the domain is overridden by the system (e.g. `com.apple.Safari`). defaults-rs attempts to read in the `Current User` + `Any Host` space for the maximum achievable domain index. This is not a threat to I/O operations so it's not really much of a caveat.

## Contributing

New pull requests and issues are always welcome. Please read the [contribution guidelines](./CONTRIBUTING.md) for more information about certain parts of the codebase and/or how to form a structured pull request.

## License

This project has been licensed under the [MIT License](./LICENSE).
