<img src="assets/logo.png" width="170px" align="right">

# 🍎 defaults-rs

#### Open-source interface to a user's defaults on macOS

![Crates.io Total Downloads](https://img.shields.io/crates/d/defaults-rs)
[![Refactor CI](https://github.com/hitblast/defaults-rs/actions/workflows/refactor.yml/badge.svg)](https://github.com/hitblast/defaults-rs/actions/workflows/refactor.yml)

<img src="assets/demo.gif">

## Key Features

- Read, write, delete, rename, import/export, and inspect preferences.
- Supports user/global/path domains.
- Supports *all* plist value types (API).
- Pretty-printing and better logging than the original `defaults` tool.
- Dynamically chooses between XML and binary PLIST data formats.
- Extremely small (<1.5k LOC).

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Rust API Usage](#rust-api-usage)
- [Why defaults-rs](#why-defaults-rs)
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
$ drs read com.apple.dock.plist tilesize   # if file exists, treated as path; else as domain
```

#### Write a key

```sh
$ drs write com.apple.dock tilesize -i 48
$ drs write com.apple.dock tilesize --int 48
$ drs write ~/Library/Preferences/com.apple.dock.plist tilesize --int 48
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

Please refer to the [API reference](https://hitblast.github.io/defaults-rs) for more information about all the available functions.

For examples, check out: [examples/](https://github.com/hitblast/defaults-rs/tree/master/examples)

## Why defaults-rs

*literally a personal take section, but here you go if you wanna listen to some yapping:*

I have made defaults-rs because I needed a handy wrapper for interacting with the preferences system the CoreFoundation framework builds up on macOS. Currently, defaults-rs serves as the [backend for cutler](https://cutlercli.github.io) when it comes to backing up and restoring system preferences.

I also found making this tool as a great way to experiment with CoreFoundation bindings and learn more about Objective-C concepts in general.

## Contributing

New pull requests and issues are always welcome. Please read the [contribution guidelines](./CONTRIBUTING.md) for more information about certain parts of the codebase and/or how to form a structured pull request.

## License

This project has been licensed under the [MIT License](./LICENSE).
