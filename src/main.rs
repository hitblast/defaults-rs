// SPDX-License-Identifier: MIT

#[cfg(feature = "cli")]
use anyhow::anyhow;
#[cfg(feature = "cli")]
use defaults_rs::cli::{build_cli, handle_subcommand};

/// main runner func
#[cfg(feature = "cli")]
fn main() {
    let matches = build_cli().get_matches();

    let result = match matches.subcommand() {
        Some((cmd, sub_m)) => match handle_subcommand(cmd, sub_m) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        },
        None => Err(anyhow!("Subcommand required")),
    };

    if let Err(e) = result {
        eprintln!("\nError: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("drs was not compiled with the \"cli\" feature enabled.")
}
