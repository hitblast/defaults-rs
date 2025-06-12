use clap::{Arg, ArgAction, Command};
use defaults_rs::preferences::Preferences;
use defaults_rs::{Domain, PrefValue, ReadResult};

#[tokio::main]
async fn main() {
    let matches = Command::new("defaults-rs")
        .about("A Rust alternative to the macOS `defaults` CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("read")
                .about("Read a value from preferences")
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::new("key").help("Preference key").index(2)),
        )
        .subcommand(
            Command::new("write")
                .about("Write a value to preferences")
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("key")
                        .help("Preference key")
                        .required(true)
                        .index(2),
                )
                .arg(
                    Arg::new("value")
                        .help("Value to write")
                        .required(true)
                        .index(3),
                )
                .arg(
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .help("Type of value (int, float, bool, string)")
                        .required(true)
                        .value_parser(["int", "float", "bool", "string"]),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a key or domain from preferences")
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::new("key").help("Preference key").index(2)),
        )
        .subcommand(
            Command::new("read-type")
                .about("Show the type for the given domain and key")
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("key")
                        .help("Preference key")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            Command::new("rename")
                .about("Rename a key in preferences")
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
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
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("path")
                        .help("Path to plist file")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            Command::new("export")
                .about("Export a domain to a plist file")
                .arg(
                    Arg::new("global")
                        .short('g')
                        .long("global")
                        .help("Use global preferences domain")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("domain")
                        .help("Domain (e.g. com.apple.dock, or any value if using -g for global)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("path")
                        .help("Path to output plist file")
                        .required(true)
                        .index(2),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("read", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let key = sub_m.get_one::<String>("key").map(|s| s.as_str());
            match Preferences::read(domain, key).await {
                Ok(ReadResult::Plist(plist_val)) => {
                    Preferences::print_apple_style(&plist_val, 0);
                    println!();
                }
                Ok(ReadResult::Value(val)) => {
                    println!("{val:?}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Some(("read-type", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let key = sub_m.get_one::<String>("key").expect("key required");
            match Preferences::read(domain, Some(key)).await {
                Ok(ReadResult::Value(val)) => {
                    let type_str = match val {
                        PrefValue::String(_) => "string",
                        PrefValue::Integer(_) => "integer",
                        PrefValue::Float(_) => "float",
                        PrefValue::Boolean(_) => "boolean",
                        PrefValue::Array(_) => "array",
                        PrefValue::Dictionary(_) => "dictionary",
                    };
                    println!("{type_str}");
                }
                Ok(ReadResult::Plist(_)) => {
                    eprintln!("Error: read-type expects a key, not a whole domain");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Some(("write", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let key = sub_m.get_one::<String>("key").expect("key required");
            let value_str = sub_m.get_one::<String>("value").expect("value required");
            let type_flag = sub_m.get_one::<String>("type").expect("type required");

            let value = match type_flag.as_str() {
                "int" => value_str
                    .parse::<i64>()
                    .map(PrefValue::Integer)
                    .unwrap_or_else(|_| {
                        eprintln!("Invalid integer value");
                        std::process::exit(1)
                    }),
                "float" => value_str
                    .parse::<f64>()
                    .map(PrefValue::Float)
                    .unwrap_or_else(|_| {
                        eprintln!("Invalid float value");
                        std::process::exit(1)
                    }),
                "bool" => match value_str.as_str() {
                    "true" | "1" => PrefValue::Boolean(true),
                    "false" | "0" => PrefValue::Boolean(false),
                    _ => {
                        eprintln!("Invalid boolean value (use true/false or 1/0)");
                        std::process::exit(1)
                    }
                },
                "string" => PrefValue::String(value_str.to_string()),
                _ => {
                    eprintln!("Unsupported type: {type_flag}");
                    std::process::exit(1)
                }
            };

            match Preferences::write(domain, key, value).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Some(("rename", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let old_key = sub_m
                .get_one::<String>("old_key")
                .expect("old_key required");
            let new_key = sub_m
                .get_one::<String>("new_key")
                .expect("new_key required");
            match Preferences::rename(domain, old_key, new_key).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Some(("import", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let path = sub_m.get_one::<String>("path").expect("path required");
            match Preferences::import(domain, path).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Some(("export", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let path = sub_m.get_one::<String>("path").expect("path required");
            match Preferences::export(domain, path).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Some(("delete", sub_m)) => {
            let domain = if sub_m.get_flag("global") {
                Domain::Global
            } else {
                let dom = sub_m.get_one::<String>("domain").expect("domain required");
                Domain::User(dom.to_string())
            };
            let key = sub_m.get_one::<String>("key").map(|s| s.as_str());
            match Preferences::delete(domain, key).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        _ => unreachable!(),
    }
}
