// SPDX-License-Identifier: MIT

//! CLI definition and argument helpers for defaults-rs.
//
// This module is responsible for:
// - Defining the command-line interface (CLI) structure using clap.
// - Specifying subcommands, arguments, and their relationships.
// - Providing helpers for argument parsing and error reporting (if needed).
//
// No business logic or backend operations are performed here.
// All CLI parsing is separated from preferences management and backend details.
use clap::{Arg, Command};

#[cfg(feature = "cli")]
pub fn build_cli() -> Command {
    use clap::ArgAction;

    let domain = |req| {
        let mut a = Arg::new("domain")
            .help("Domain (e.g. com.example.app / -g / NSGlobalDomain) or a system-recognized plist path")
            .index(1)
            .allow_hyphen_values(true);
        if req {
            a = a.required(true)
        }
        a
    };

    let key = |req| {
        let mut a = Arg::new("key").help("Preference key").index(2);
        if req {
            a = a.required(true)
        }
        a
    };

    let path = Arg::new("path")
        .help("Path to plist file")
        .required(true)
        .index(2);

    Command::new("defaults-rs")
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("read")
                .about("Read a value")
                .arg(domain(false))
                .arg(key(false)),
        )
        .subcommand(
            Command::new("read-type")
                .about("Show type")
                .arg(domain(true))
                .arg(key(true)),
        )
        .subcommand(
            Command::new("write")
                .about("Write value")
                .arg(domain(true))
                .arg(key(true))
                .arg(
                    Arg::new("force")
                        .short('F')
                        .long("force")
                        .help("Disable domain check")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("int")
                        .short('i')
                        .long("int")
                        .num_args(1)
                        .value_name("VALUE")
                        .help("Write an integer value")
                        .conflicts_with_all(["float", "bool", "string"]),
                )
                .arg(
                    Arg::new("float")
                        .short('f')
                        .long("float")
                        .num_args(1)
                        .value_name("VALUE")
                        .help("Write a float value")
                        .conflicts_with_all(["int", "bool", "string"]),
                )
                .arg(
                    Arg::new("bool")
                        .short('b')
                        .long("bool")
                        .num_args(1)
                        .value_name("VALUE")
                        .help("Write a boolean value (true/false/1/0/yes/no)")
                        .conflicts_with_all(["int", "float", "string"]),
                )
                .arg(
                    Arg::new("string")
                        .short('s')
                        .long("string")
                        .num_args(1)
                        .value_name("VALUE")
                        .conflicts_with_all(["int", "float", "bool"]),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete key/domain")
                .arg(domain(true))
                .arg(key(false)),
        )
        .subcommand(
            Command::new("rename")
                .about("Rename key")
                .arg(domain(true))
                .arg(
                    Arg::new("old_key")
                        .help("Old/original key name")
                        .required(true)
                        .index(2),
                )
                .arg(
                    Arg::new("new_key")
                        .help("New key name")
                        .required(true)
                        .index(3),
                ),
        )
        .subcommand(
            Command::new("import")
                .about("Import plist")
                .arg(domain(true))
                .arg(&path),
        )
        .subcommand(
            Command::new("export")
                .about("Export plist")
                .arg(domain(true))
                .arg(path),
        )
        .subcommand(
            Command::new("domains").about("List domains").arg(
                Arg::new("no-fuzzy")
                    .short('n')
                    .long("no-fuzzy")
                    .help("Disable fuzzy-picker")
                    .action(ArgAction::SetTrue),
            ),
        )
        .subcommand(
            Command::new("find").about("Search all domains").arg(
                Arg::new("word")
                    .help("Word to search for (case-insensitive)")
                    .required(true)
                    .index(1),
            ),
        )
}

#[cfg(feature = "cli")]
pub fn get_required_arg<'a>(sub_m: &'a clap::ArgMatches, name: &str) -> &'a str {
    sub_m
        .get_one::<String>(name)
        .map(String::as_str)
        .unwrap_or_else(|| {
            eprintln!("Error: {name} required");
            std::process::exit(1);
        })
}

#[cfg(feature = "cli")]
pub fn print_result<T, E: std::fmt::Display>(res: Result<T, E>)
where
    T: std::fmt::Debug,
{
    match res {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Error: {e}"),
    }
}
