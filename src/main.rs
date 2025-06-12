use clap::{Arg, ArgMatches, Command};
use defaults_rs::preferences::Preferences;
use defaults_rs::{Domain, PrefValue, ReadResult};

#[tokio::main]
async fn main() {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some((cmd, sub_m)) => handle_subcommand(cmd, sub_m).await,
        None => unreachable!("Subcommand required"),
    }
}

fn build_cli() -> Command {
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
        .index(4)
        .value_parser(["int", "float", "bool", "string"])
}

fn path_arg() -> Arg {
    Arg::new("path")
        .help("Path to plist file")
        .required(true)
        .index(2)
}

fn parse_domain(sub_m: &ArgMatches) -> Domain {
    let dom = sub_m.get_one::<String>("domain").expect("domain required");
    match dom.as_str() {
        "-g" | "NSGlobalDomain" => Domain::Global,
        other => Domain::User(other.to_string()),
    }
}

async fn handle_subcommand(cmd: &str, sub_m: &ArgMatches) {
    let domain = parse_domain(sub_m);

    match cmd {
        "read" => {
            let key = sub_m.get_one::<String>("key").map(String::as_str);
            match Preferences::read(domain, key).await {
                Ok(ReadResult::Plist(plist_val)) => {
                    Preferences::print_apple_style(&plist_val, 0);
                    println!();
                }
                Ok(ReadResult::Value(val)) => println!("{val:?}"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "read-type" => {
            let key = sub_m.get_one::<String>("key").expect("key required");
            match Preferences::read(domain, Some(key)).await {
                Ok(ReadResult::Value(val)) => println!("{}", val.type_name()),
                Ok(ReadResult::Plist(_)) => {
                    eprintln!("Error: read-type expects a key, not a whole domain")
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "write" => {
            let key = sub_m.get_one::<String>("key").unwrap();
            let value_str = sub_m.get_one::<String>("value").unwrap();
            let type_flag = sub_m.get_one::<String>("type").unwrap();

            let value = PrefValue::from_str(type_flag, value_str).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1)
            });

            match Preferences::write(domain, key, value).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "delete" => {
            let key = sub_m.get_one::<String>("key").map(String::as_str);
            match Preferences::delete(domain, key).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "rename" => {
            let old_key = sub_m.get_one::<String>("old_key").unwrap();
            let new_key = sub_m.get_one::<String>("new_key").unwrap();
            match Preferences::rename(domain, old_key, new_key).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "import" => {
            let path = sub_m.get_one::<String>("path").unwrap();
            match Preferences::import(domain, path).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "export" => {
            let path = sub_m.get_one::<String>("path").unwrap();
            match Preferences::export(domain, path).await {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        _ => unreachable!(),
    }
}
