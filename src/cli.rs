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
                .arg(
                    Arg::new("int")
                        .help("Write an integer value")
                        .short('i')
                        .long("int")
                        .num_args(1)
                        .value_name("VALUE")
                        .conflicts_with_all(&["float", "bool", "string"]),
                )
                .arg(
                    Arg::new("float")
                        .help("Write a float value")
                        .short('f')
                        .long("float")
                        .num_args(1)
                        .value_name("VALUE")
                        .conflicts_with_all(&["int", "bool", "string"]),
                )
                .arg(
                    Arg::new("bool")
                        .help("Write a boolean value (true/false/1/0)")
                        .short('b')
                        .long("bool")
                        .num_args(1)
                        .value_name("VALUE")
                        .conflicts_with_all(&["int", "float", "string"]),
                )
                .arg(
                    Arg::new("string")
                        .help("Write a string value")
                        .short('s')
                        .long("string")
                        .num_args(1)
                        .value_name("VALUE")
                        .conflicts_with_all(&["int", "float", "bool"]),
                ),
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
        .subcommand(Command::new("domains").about("List all available preference domains"))
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

fn path_arg() -> Arg {
    Arg::new("path")
        .help("Path to plist file")
        .required(true)
        .index(2)
}
