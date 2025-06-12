//! CLI definition and argument helpers for defaults-rs.

use clap::{Arg, Command};

pub fn build_cli() -> Command {
    Command::new("defaults-rs")
        .about(
            "Command line interface to a user's defaults. Substitute for original `defaults` CLI.",
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("read")
                .about("Read a value from preferences")
                .arg(domain_arg())
                .arg(key_arg(false)),
        )
        .subcommand(
            Command::new("read-type")
                .about("Show the type for the given domain and key")
                .arg(domain_arg())
                .arg(key_arg(true)),
        )
        .subcommand(
            Command::new("write")
                .about("Write a value to preferences")
                .arg(domain_arg())
                .arg(key_arg(true))
                .arg(value_arg())
                .arg(type_arg()),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a key or domain from preferences")
                .arg(domain_arg())
                .arg(key_arg(false)),
        )
        .subcommand(
            Command::new("rename")
                .about("Rename a key in preferences")
                .arg(domain_arg())
                .arg(
                    Arg::new("old_key")
                        .help("Old key name")
                        .required(true)
                        .index(3),
                )
                .arg(
                    Arg::new("new_key")
                        .help("New key name")
                        .required(true)
                        .index(4),
                ),
        )
        .subcommand(
            Command::new("import")
                .about("Import a plist file into a domain")
                .arg(domain_arg())
                .arg(path_arg()),
        )
        .subcommand(
            Command::new("export")
                .about("Export a domain to a plist file")
                .arg(domain_arg())
                .arg(path_arg()),
        )
}

fn domain_arg() -> Arg {
    Arg::new("domain")
        .help("Domain (e.g. com.apple.dock). Use '-g' or 'NSGlobalDomain' for global domain")
        .required(true)
        .index(1)
        .allow_hyphen_values(true)
}

fn key_arg(required: bool) -> Arg {
    let mut arg = Arg::new("key").help("Preference key").index(2);
    if required {
        arg = arg.required(true);
    }
    arg
}

fn value_arg() -> Arg {
    Arg::new("value")
        .help("Value to write")
        .required(true)
        .index(3)
}

fn type_arg() -> Arg {
    Arg::new("type")
        .short('t')
        .long("type")
        .help("Type of value (int, float, bool, string)")
        .required(true)
        .value_parser(["int", "float", "bool", "string"])
}

fn path_arg() -> Arg {
    Arg::new("path")
        .help("Path to plist file")
        .required(true)
        .index(2)
}
