# Contribution Guidelines

This is the standard contribution/development guidelines for the project. You may follow these to get a hold of the project quickly.

## Table of Contents

- [Getting Started](#getting-started)
- [Production Release Workflow](#production-release-workflow)
- [Pull Request Guidelines](#pull-request-guidelines)
- [License](#licensing)

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

## Production Release Workflow

This chain of commands can be used to fully test and build the final product.

### Testing

```bash
# raw command
$ cargo fmt --all -- --check && cargo clippy && cargo build
```

> [!NOTE]
> The unit tests in the CI workflow are done using an **Apple Silicon M1 (3-core)** runner provided by GitHub Actions. See [this page](https://docs.github.com/en/actions/using-github-hosted-runners/using-github-hosted-runners/about-github-hosted-runners#supported-runners-and-hardware-resources) in GitHub's documentation for more information on all the runners. If the runners used in this project get outdated and don't get a bump, you may suggest one through [GitHub Issues](https://github.com/hitblast/defaults-rs/issues/new).

### Build Reproduction

You can easily create a release build for defaults-rs using the following command:

```bash
$ cargo build --release --verbose --locked
```

The major part of the release automation is currently done with [GitHub Actions]() via the [following workflow](./.github/workflows/release.yml) so, you can have a look at it to view the entire pipeline.

The unit testing is done via [this workflow.](./.github/workflows/tests.yml)

### Code Formatting

`defaults-rs` uses basic Rust formatting for code reliability and maintainability. This ensures that the codebase remains clean, readable, and consistent across different contributors.

Simply run the following command to format the code:

```bash
$ cargo fmt --all
```

## Pull Request Guidelines

Before submitting a pull request, please ensure the following:

- Your code is well-documented and follows the established coding standards.
- The repository is correctly forked and your working branch is up-to-date with the latest changes from the main branch.
- All tests pass locally, and you have verified that your changes do not introduce regressions.
- If your pull request fixes an issue, mention the issue number in your PR description (e.g., Fixes #123).
- For larger changes, consider discussing your approach by opening an issue first.

Pull requests and issues must have the following pattern:

```
(<type>) <title>
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
