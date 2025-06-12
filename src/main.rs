use clap::ArgMatches;
use defaults_rs::build_cli;
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

fn parse_domain(sub_m: &ArgMatches) -> Domain {
    let dom = sub_m.get_one::<String>("domain").expect("domain required");
    match dom.as_str() {
        "-g" | "NSGlobalDomain" => Domain::Global,
        other => Domain::User(other.to_string()),
    }
}

// Helper to print Ok(()) or error
fn print_result<T, E: std::fmt::Display>(res: Result<T, E>)
where
    T: std::fmt::Debug,
{
    match res {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Error: {e}"),
    }
}

// Helper to get a required argument
fn get_required_arg<'a>(sub_m: &'a ArgMatches, name: &str) -> &'a str {
    sub_m
        .get_one::<String>(name)
        .map(String::as_str)
        .unwrap_or_else(|| {
            eprintln!("Error: {name} required");
            std::process::exit(1);
        })
}

async fn handle_subcommand(cmd: &str, sub_m: &ArgMatches) {
    match cmd {
        "domains" => match Preferences::list_domains().await {
            Ok(domains) => {
                for dom in domains {
                    println!("{dom}");
                }
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        _ => {
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
                    let key = get_required_arg(sub_m, "key");
                    match Preferences::read(domain, Some(key)).await {
                        Ok(ReadResult::Value(val)) => println!("{}", val.type_name()),
                        Ok(ReadResult::Plist(_)) => {
                            eprintln!("Error: read-type expects a key, not a whole domain")
                        }
                        Err(e) => eprintln!("Error: {e}"),
                    }
                }
                "write" => {
                    let key = get_required_arg(sub_m, "key");

                    // Detect which type flag was used and get the value
                    let (type_flag, value_str) = if let Some(val) = sub_m.get_one::<String>("int") {
                        ("int", val)
                    } else if let Some(val) = sub_m.get_one::<String>("float") {
                        ("float", val)
                    } else if let Some(val) = sub_m.get_one::<String>("bool") {
                        ("bool", val)
                    } else if let Some(val) = sub_m.get_one::<String>("string") {
                        ("string", val)
                    } else {
                        eprintln!(
                            "Error: You must specify one of -int, -float, -bool, or -string for the value type."
                        );
                        std::process::exit(1);
                    };

                    let value = PrefValue::from_str(type_flag, value_str).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1)
                    });

                    print_result(Preferences::write(domain, key, value).await);
                }
                "delete" => {
                    let key = sub_m.get_one::<String>("key").map(String::as_str);
                    print_result(Preferences::delete(domain, key).await);
                }
                "rename" => {
                    let old_key = get_required_arg(sub_m, "old_key");
                    let new_key = get_required_arg(sub_m, "new_key");
                    print_result(Preferences::rename(domain, old_key, new_key).await);
                }
                "import" => {
                    let path = get_required_arg(sub_m, "path");
                    print_result(Preferences::import(domain, path).await);
                }
                "export" => {
                    let path = get_required_arg(sub_m, "path");
                    print_result(Preferences::export(domain, path).await);
                }
                _ => unreachable!(),
            }
        }
    }
}
