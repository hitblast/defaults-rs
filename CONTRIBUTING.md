# Contribution Guidelines

This is the standard contribution/development guidelines for the project. You may follow these to get a hold of the project quickly.

## Table of Contents

- [Getting Started](#getting-started)
  - [Cloning the repository](#cloning-the-repository)
  - [Preparing the environment](#preparing-the-environment)
  - [Project Structure](#project-structure)
- [Production Release Workflow](#production-release-workflow)
  - [Refactoring](#refactoring)
  - [Build Reproduction](#build-reproduction)
- [Pull Request Guidelines](#pull-request-guidelines)
- [License](#license)

## Getting Started

The commonplace of contributing is to first clone the repository and install the dependencies.

The prerequisites are as follows:

- [Rust](https://www.rust-lang.org/tools/install) (`defaults-rs` is configured to use the 2024 edition of the language)
- A Mac (preferably with [Apple Silicon](https://support.apple.com/en-us/HT211814)) for rapid development

### Cloning the repository

Once you have ensured the prerequisites, fork the repository [from here](https://github.com/hitblast/defaults-rs/fork) and clone it using the following command:

```bash
# https
$ git clone https://github.com/<your_username>/defaults-rs.git

# ssh
$ git clone git@github.com:<your_username>/defaults-rs.git
```

Replace `<your_username>` with your GitHub username.

### Preparing the environment

Working on this project will require a few Rust components beforehand:

- [clippy](https://github.com/rust-lang/rust-clippy)
- [rustfmt](https://github.com/rust-lang/rustfmt)

### Project Structure

The main source code for `defaults-rs` is located in the `src/` directory. Here is an overview of the project's structure:

```
defaults-rs/
├── src/
│   ├── cli.rs            # CLI definition and argument helpers (clap-based)
│   ├── core/
│   │   ├── convert.rs    # CoreFoundation <-> PrefValue conversion logic
│   │   ├── foundation.rs # CoreFoundation-based preferences backend
│   │   ├── mod.rs        # Core module declarations
│   │   └── types.rs      # PrefValue type definitions
│   ├── lib.rs            # Library API entry point
│   ├── main.rs           # CLI entry point
│   ├── preferences/
│   │   ├── convert.rs    # Plist <-> PrefValue conversion logic
│   │   ├── mod.rs        # Preferences API implementation
│   │   └── types.rs      # Domain and FindMatch types
│   └── prettifier.rs     # Apple-style pretty-printing for CLI output
├── Cargo.toml            # Rust crate manifest
├── LICENSE               # MIT License
└── README.md             # Project documentation
```

- **src/cli.rs**: Defines the command-line interface, subcommands, and argument parsing.
- **src/core/**: Contains low-level CoreFoundation integration and type conversions.
- **src/preferences/**: Implements business logic for reading, writing, importing/exporting, and batch operations on preferences.
- **src/prettifier.rs**: Formats output in Apple-style for CLI display.
- **src/lib.rs**: Exposes the public library API.
- **src/main.rs**: Entry point for the CLI application.

## Production Release Workflow

This chain of commands can be used to fully test and build the final product.

### Refactoring

Review [this GitHub Actions workflow](./.github/workflows/refactor.yml) for refactoring reference.

> [!NOTE]
> All the workflows in this repository are run on an **Apple Silicon M1 (3-core)** runner provided by GitHub Actions. See [this page](https://docs.github.com/en/actions/using-github-hosted-runners/using-github-hosted-runners/about-github-hosted-runners#supported-runners-and-hardware-resources) in GitHub's documentation for more information on all the runners. If the runners used in this project get outdated and don't get a bump, you may suggest one through [GitHub Issues](https://github.com/hitblast/defaults-rs/issues/new).

### Build Reproduction

You can easily create a release build for defaults-rs using the following command:

```bash
$ cargo build --release --verbose --locked
```

The major part of the release automation is currently done with [GitHub Actions]() via the [following workflow](./.github/workflows/release.yml) so, you can have a look at it to view the entire pipeline.

## Pull Request Guidelines

Before submitting a pull request, please ensure the following:

- Your code is well-documented and follows the established coding standards.
- The repository is correctly forked and your working branch is up-to-date with the latest changes from the main branch.
- All tests pass locally, and you have verified that your changes do not introduce regressions.
- If your pull request fixes an issue, mention the issue number in your PR description (e.g., Fixes #123).
- For larger changes, consider discussing your approach by opening an issue first.

Pull requests and issues must have the following pattern:

```
<type>: <title>
```

Possible types include:

- feat: New feature or enhancement
- fix: Bug fix
- docs: Documentation update
- style: Code style or formatting change
- refactor: Code refactoring without changing functionality
- test: Test-related changes
- chore: Maintenance or administrative tasks

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/hitblast/defaults-rs/blob/main/LICENSE) file for details.
