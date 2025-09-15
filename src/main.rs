use std::path::PathBuf;

use clap::ArgMatches;
use defaults_rs::build_cli;
use defaults_rs::cli::{get_required_arg, print_result};
use defaults_rs::preferences::Preferences;
use defaults_rs::prettifier::Prettifier;
use defaults_rs::{Domain, PrefValue, ReadResult};
use tokio::fs;

/// main runner func
#[tokio::main]
async fn main() {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some((cmd, sub_m)) => handle_subcommand(cmd, sub_m).await,
        None => unreachable!("Subcommand required"),
    }
}

/// Returns a domain object based on the kind of the argument that is passed.
async fn parse_domain_or_path(sub_m: &ArgMatches) -> Domain {
    let domain = sub_m.get_one::<String>("domain").expect("domain required");
    let path = PathBuf::from(domain);

    // Try as-is
    if fs::try_exists(&path).await.unwrap() {
        return Domain::Path(path);
    }

    // Try with .plist extension if not already present
    if path.extension().and_then(|e| e.to_str()) != Some("plist") {
        let mut with_ext = path.clone();
        with_ext.set_extension("plist");
        if with_ext.exists() {
            return Domain::Path(with_ext);
        }
    }

    // Fallback to domain logic
    match domain.as_str() {
        "-g" | "NSGlobalDomain" | "-globalDomain" => Domain::Global,
        other => {
            if other.contains("..")
                || other.contains('/')
                || other.contains('\\')
                || !other
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
            {
                eprintln!("Error: invalid domain or plist path: {other}");
                std::process::exit(1);
            }
            Domain::User(other.to_string())
        }
    }
}

/// Function to handle subcommand runs.
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
        "find" => {
            let word = get_required_arg(sub_m, "word");
            match Preferences::find(word).await {
                Ok(results) => {
                    for (domain, matches) in results {
                        println!("Found {} matches for domain `{}`:", matches.len(), domain);
                        for m in matches {
                            println!("    {} = {}", m.key_path, m.value);
                        }
                        println!();
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        _ => {
            let domain = parse_domain_or_path(sub_m).await;
            match cmd {
                "read" => {
                    let key = sub_m.get_one::<String>("key").map(String::as_str);
                    match Preferences::read(domain, key).await {
                        Ok(ReadResult::Plist(plist_val)) => {
                            Prettifier::print_apple_style(&plist_val, 0);
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
                            eprintln!("Error: read-type expects a key, not a whole domain or plist")
                        }
                        Err(e) => eprintln!("Error: {e}"),
                    }
                }
                "write" => {
                    let key = get_required_arg(sub_m, "key");

                    // Detect which type flag was used and get the value.
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
                            "Error: You must specify one of --int, --float, --bool, or --string for the value type."
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
