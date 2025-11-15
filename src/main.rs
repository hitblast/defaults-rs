// SPDX-License-Identifier: MIT

#[cfg(feature = "cli")]
mod prettifier;
#[cfg(feature = "cli")]
use prettifier::prettify;

#[cfg(feature = "cli")]
use anyhow::{Context, Result};
#[cfg(feature = "cli")]
use anyhow::{anyhow, bail};
#[cfg(feature = "cli")]
use clap::ArgMatches;
#[cfg(feature = "cli")]
use defaults_rs::{
    Domain, PrefValue, Preferences, build_cli,
    cli::{get_required_arg, print_result},
};
#[cfg(feature = "cli")]
use skim::prelude::*;
#[cfg(feature = "cli")]
use std::io::Cursor;
#[cfg(feature = "cli")]
use std::path::Path;

/// main runner func
#[cfg(feature = "cli")]
fn main() {
    let matches = build_cli().get_matches();

    let result = match matches.subcommand() {
        Some((cmd, sub_m)) => match handle_subcommand(cmd, sub_m) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        },
        None => unreachable!("Subcommand required"),
    };

    if let Err(e) = result {
        eprintln!("\nError: {e}");
        std::process::exit(1);
    }
}

/// Returns a domain object based on the kind of the argument that is passed.
#[cfg(feature = "cli")]
fn parse_domain_or_path(sub_m: &ArgMatches, force: bool) -> Result<Domain> {
    use defaults_rs::Domain;

    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve home directory"))?;

    let mut domain = sub_m
        .get_one::<String>("domain")
        .context("domain argument is required")?
        .to_string();

    // filepath check
    if let Ok(path) = Path::new(domain.as_str()).canonicalize()
        && path.is_file()
        && (path.starts_with(format!(
            "{}/Library/Preferences/",
            home_dir.to_string_lossy()
        )) || path.starts_with("/Library/Preferences/")
            || path.starts_with("/System/Library/Preferences/")
            || path
                == Path::new(&format!(
                    "{}/Library/Preferences/.GlobalPreferences.plist",
                    home_dir.to_string_lossy()
                ))
            || path == Path::new("/Library/Preferences/.GlobalPreferences.plist"))
    {
        domain = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("could not get file stem"))?
            .to_string();
    }

    // domain check
    match domain.strip_suffix(".plist").unwrap_or(&domain) {
        "-g" | "NSGlobalDomain" | "-globalDomain" => Ok(Domain::Global),
        other => {
            if other.contains("..")
                || other.contains('/')
                || other.contains('\\')
                || !other
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
            {
                bail!("Invalid domain or plist path: {other}");
            }

            if !force
                && !Preferences::list_domains()?
                    .iter()
                    .any(|dom| dom.to_string() == other)
            {
                bail!("Domain '{domain}' not found!.")
            }

            Ok(Domain::User(other.to_string()))
        }
    }
}

/// Returns a string representation of a preference value based on the typeflag passed.s
#[cfg(feature = "cli")]
fn from_typeflag_str(type_flag: &str, s: &str) -> Result<PrefValue> {
    match type_flag {
        "int" => {
            let val = s
                .parse::<i64>()
                .map_err(|e| anyhow!("Failed to parse int: {e}"))?;
            Ok(PrefValue::Integer(val))
        }
        "float" => {
            let val = s
                .parse::<f64>()
                .map_err(|e| anyhow!("Failed to parse float: {e}"))?;
            Ok(PrefValue::Float(val))
        }
        "bool" => match s {
            "true" | "1" | "yes" => Ok(PrefValue::Boolean(true)),
            "false" | "0" | "no" => Ok(PrefValue::Boolean(false)),
            _ => bail!("Invalid boolean value (use true/false or 1/0)"),
        },
        "string" => Ok(PrefValue::String(s.to_string())),
        other => bail!("Unsupported type: {other}"),
    }
}

/// Fuzzy-picking helper for the CLI.
#[cfg(feature = "cli")]
fn pick_one(prompt: &str, items: &[String]) -> Option<String> {
    let item_reader = SkimItemReader::default();
    let skim_items = item_reader.of_bufread(Cursor::new(items.join("\n")));

    let options = SkimOptionsBuilder::default()
        .prompt(prompt.to_string())
        .color(Some("bw".to_string()))
        .case(CaseMatching::Smart)
        .multi(false)
        .build()
        .unwrap();

    let out = Skim::run_with(&options, Some(skim_items));

    let out = match out {
        Some(o) if !o.is_abort => o,
        _ => return None,
    };

    if out.selected_items.first().is_some() {
        Some(out.selected_items.first().unwrap().output().to_string())
    } else {
        None
    }
}

/// Function to handle subcommand runs.
#[cfg(feature = "cli")]
fn handle_subcommand(cmd: &str, sub_m: &ArgMatches) -> Result<()> {
    match cmd {
        "domains" => {
            let domains = Preferences::list_domains()?;
            let domains_str: Vec<String> = domains.iter().map(|f| f.to_string()).collect();

            if sub_m.get_flag("no-fuzzy") {
                for dom in domains {
                    println!("{dom}");
                }
            } else {
                let picker = pick_one(
                    "Viewing list of domains. Use arrow keys to navigate: ",
                    &domains_str,
                );

                if let Some(picked_domain) = picker {
                    println!("Domain: {picked_domain} (is {})", {
                        match domains
                            .iter()
                            .find(|d| d.to_string() == picked_domain)
                            .context("Domain-type checker failed.")?
                        {
                            Domain::User(_) => "user domain",
                            Domain::Global => "global domain",
                        }
                    })
                }
            }

            Ok(())
        }
        "find" => {
            let word = get_required_arg(sub_m, "word");
            let results = Preferences::find(word)?;
            for (domain, matches) in results {
                println!("Found {} matches for domain `{}`:", matches.len(), domain);
                for m in matches {
                    println!("    {} = {}", m.key, m.value);
                }
                println!();
            }
            Ok(())
        }
        "write" => {
            let force = sub_m.get_flag("force");

            let domain: Domain = if let Ok(val) = parse_domain_or_path(sub_m, force) {
                val
            } else {
                bail!("Could not write to non-existing domain. If intentional, use -F/--force.")
            };

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
                bail!(
                    "You must specify one of --int, --float, --bool, or --string for the value type."
                )
            };

            let value = from_typeflag_str(type_flag, value_str)?;
            print_result(Preferences::write(domain, key, value));
            Ok(())
        }
        "read" => {
            let input_domain = sub_m.get_one::<String>("domain");
            let input_key = sub_m.get_one::<String>("key");

            let domain: Domain = if let Ok(val) = parse_domain_or_path(sub_m, false) {
                val
            } else if input_domain.is_none() && input_key.is_none() {
                let domains = Preferences::list_domains()?;
                let domains_str: Vec<String> = domains.iter().map(|f| f.to_string()).collect();

                let chosen = pick_one(
                    "Select a proper domain to read. Use arrow keys to navigate: ",
                    &domains_str,
                )
                .context("domain argument is required!")?;

                domains
                    .into_iter()
                    .find(|d| d.to_string() == chosen)
                    .unwrap()
            } else {
                bail!("invalid domain passed.")
            };

            let val = if let Some(key) = sub_m.get_one::<String>("key").map(String::as_str) {
                Preferences::read(domain, key)?
            } else {
                Preferences::read_domain(domain)?
            };

            println!("{}", prettify(&val, 0));
            Ok(())
        }
        "read-type" => {
            let domain: Domain = parse_domain_or_path(sub_m, false)?;
            let key = get_required_arg(sub_m, "key");
            let val = Preferences::read(domain, key)?;

            println!("Type is {}", val.get_type());
            Ok(())
        }
        "delete" => {
            let key = sub_m.get_one::<String>("key").map(String::as_str);
            let domain: Domain = parse_domain_or_path(sub_m, false)?;

            if let Some(key) = key {
                Preferences::delete(domain, key)
            } else {
                Preferences::delete_domain(domain)
            }
        }
        "rename" => {
            let domain: Domain = parse_domain_or_path(sub_m, false)?;
            let old_key = get_required_arg(sub_m, "old_key");
            let new_key = get_required_arg(sub_m, "new_key");

            print_result(Preferences::rename(domain, old_key, new_key));
            Ok(())
        }
        "import" => {
            let domain: Domain = parse_domain_or_path(sub_m, false)?;
            let path = get_required_arg(sub_m, "path");

            print_result(Preferences::import(domain, path));
            Ok(())
        }
        "export" => {
            let domain: Domain = parse_domain_or_path(sub_m, false)?;
            let path = get_required_arg(sub_m, "path");

            print_result(Preferences::export(domain, path));
            Ok(())
        }
        _ => unreachable!(),
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("drs was not compiled with the \"cli\" feature enabled.")
}
