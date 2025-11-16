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

/// Extract the proper PrefValue to be writtenf rom the passed typeflag.
///
/// This is primarily used in the write command for determining types.
#[cfg(feature = "cli")]
fn extract_prefvalue_from_args(sub_m: &ArgMatches) -> Result<PrefValue> {
    if let Some(val) = sub_m.get_one::<String>("int") {
        let val = val
            .parse::<i64>()
            .map_err(|e| anyhow!("Failed to parse int: {e}"))?;
        Ok(PrefValue::Integer(val))
    } else if let Some(val) = sub_m.get_one::<String>("float") {
        let val = val
            .parse::<i64>()
            .map_err(|e| anyhow!("Failed to parse int: {e}"))?;
        Ok(PrefValue::Integer(val))
    } else if let Some(val) = sub_m.get_one::<String>("bool") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(PrefValue::Boolean(true)),
            "false" | "0" | "no" => Ok(PrefValue::Boolean(false)),
            _ => bail!("Invalid boolean value (use true/false, yes/no or 1/0)"),
        }
    } else if let Some(val) = sub_m.get_many::<String>("array") {
        let val: Vec<PrefValue> = val
            .into_iter()
            .map(|f| PrefValue::String(f.to_string()))
            .collect();

        Ok(PrefValue::Array(val))
    } else if let Some(val) = sub_m.get_one::<String>("string") {
        Ok(PrefValue::String(val.to_string()))
    } else {
        bail!(
            "You must specify one of --int, --float, --bool, --array or --string for the value type."
        )
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
        .expect("Failed to build options for picker.");

    let out = Skim::run_with(&options, Some(skim_items));

    let out = match out {
        Some(o) if !o.is_abort => o,
        _ => return None,
    };

    out.selected_items
        .first()
        .map(|item| item.output().to_string())
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

            let value = extract_prefvalue_from_args(sub_m)?;
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
                    .ok_or_else(|| anyhow!("Selected domain not found in available domains!"))?
            } else {
                bail!(
                    "Invalid domain passed: {:?}. Please provide a valid domain name (e.g., 'com.example.app'), or use the fuzzy picker by omitting both domain and key arguments.",
                    input_domain
                )
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

            let result = if let Some(key) = key {
                Preferences::delete(domain, key)
            } else {
                Preferences::delete_domain(domain)
            };

            print_result(result);
            Ok(())
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
