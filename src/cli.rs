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
#[cfg(feature = "cli")]
use crate::Domain;
#[cfg(feature = "cli")]
use crate::prettifier::prettify;
#[cfg(feature = "cli")]
use crate::{PrefValue, Preferences};
#[cfg(feature = "cli")]
use anyhow::{Context, Result, anyhow, bail};
#[cfg(feature = "cli")]
use clap::{Arg, ArgMatches, Command};
#[cfg(feature = "cli")]
use skim::prelude::*;
#[cfg(feature = "cli")]
use std::io::Cursor;
#[cfg(feature = "cli")]
use std::path::Path;

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
                        .conflicts_with_all(["float", "bool", "string", "array"]),
                )
                .arg(
                    Arg::new("float")
                        .short('f')
                        .long("float")
                        .num_args(1)
                        .value_name("VALUE")
                        .help("Write a float value")
                        .conflicts_with_all(["int", "bool", "string", "array"]),
                )
                .arg(
                    Arg::new("bool")
                        .short('b')
                        .long("bool")
                        .num_args(1)
                        .value_name("VALUE")
                        .help("Write a boolean value (true/false/1/0/yes/no)")
                        .conflicts_with_all(["int", "float", "string", "array"]),
                )
                .arg(
                    Arg::new("string")
                        .short('s')
                        .long("string")
                        .num_args(1)
                        .value_name("VALUE")
                        .help("Write a string value")
                        .conflicts_with_all(["int", "float", "bool", "array"]),
                )
                .arg(
                    Arg::new("array")
                        .short('a')
                        .long("array")
                        .value_name("VALUE")
                        .num_args(1..)
                        .help("Write an array value")
                        .conflicts_with_all(["int", "float", "bool", "string"]),
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

/// Returns a domain object based on the kind of the argument that is passed.
#[cfg(feature = "cli")]
fn parse_domain_or_path(sub_m: &ArgMatches, force: bool) -> Result<Domain> {
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
            || path.starts_with("/System/Library/Preferences/"))
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

/// Extract the proper PrefValue to be writtem from the passed typeflag.
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
            .parse::<f64>()
            .map_err(|e| anyhow!("Failed to parse int: {e}"))?;
        Ok(PrefValue::Float(val))
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

/// Returns a required argument from the CLI.
#[cfg(feature = "cli")]
fn get_required_arg<'a>(sub_m: &'a clap::ArgMatches, name: &str) -> &'a str {
    sub_m
        .get_one::<String>(name)
        .map(String::as_str)
        .unwrap_or_else(|| {
            eprintln!("Error: {name} required");
            std::process::exit(1);
        })
}

/// Fuzzy-picking helper for the CLI.
#[cfg(feature = "cli")]
fn pick_one(prompt: &str, items: &[String]) -> Result<Option<String>> {
    let item_reader = SkimItemReader::default();
    let skim_items = item_reader.of_bufread(Cursor::new(items.join("\n")));

    let options = SkimOptionsBuilder::default()
        .prompt(prompt.to_string())
        .color(Some("bw".to_string()))
        .case(CaseMatching::Smart)
        .multi(false)
        .build()
        .context("Failed to build fuzzy-picker options; internal error in pick_one().")?;

    let out = Skim::run_with(&options, Some(skim_items));

    let out = match out {
        Some(o) if !o.is_abort => o,
        _ => return Ok(None),
    };

    Ok(out
        .selected_items
        .first()
        .map(|item| item.output().to_string()))
}

/// Function to handle subcommand runs.
#[cfg(feature = "cli")]
pub fn handle_subcommand(cmd: &str, sub_m: &ArgMatches) -> Result<()> {
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
                )?;

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
            Preferences::write(domain, key, value)
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
                )?;

                if let Some(chosen) = chosen {
                    domains
                        .into_iter()
                        .find(|d| d.to_string() == chosen)
                        .context("Unexpected domain mismatch here.")?
                } else {
                    bail!("No domain selected.")
                }
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

            Preferences::rename(domain, old_key, new_key)
        }
        "import" => {
            let domain: Domain = parse_domain_or_path(sub_m, false)?;
            let path = get_required_arg(sub_m, "path");

            Preferences::import(domain, path)
        }
        "export" => {
            let domain: Domain = parse_domain_or_path(sub_m, false)?;
            let path = get_required_arg(sub_m, "path");

            Preferences::export(domain, path)
        }
        _ => bail!("Not a proper subcommand."),
    }
}
