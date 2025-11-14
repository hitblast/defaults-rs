// SPDX-License-Identifier: MIT

//! Preferences API for defaults-rs.
//!
//! This module implements all business logic for reading, writing, deleting, importing/exporting,
//! batch operations, and pretty-printing macOS preferences (plist files).
//!
//! It acts as the main interface between the CLI/library and the backend (CoreFoundation or file-based).

mod convert;
pub mod types;

use anyhow::{Context, Result, bail};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::Cursor,
    path::PathBuf,
};

use crate::{
    Domain, FindMatch, PrefValue,
    preferences::convert::{plist_to_prefvalue, prefvalue_to_plist},
};
use plist::Value;

/// Backend selection for preferences (CoreFoundation vs File)
use crate::core::foundation;

/// Provides operations for reading, writing, deleting, and managing
/// macOS plist preference files in user or global domains.
pub struct Preferences;

impl Preferences {
    /// List all available domains in ~/Library/Preferences.
    pub fn list_domains() -> Result<Vec<Domain>> {
        let list = foundation::list_domains()?;

        let domains: Vec<Domain> = list.iter().map(|f| Domain::User(f.to_string())).collect();
        Ok(domains)
    }

    /// Search all domains for keys or values containing the given word (case-insensitive).
    pub fn find(word: &str) -> Result<BTreeMap<String, Vec<FindMatch>>> {
        let word_lower = word.to_lowercase();
        let mut results: BTreeMap<String, Vec<FindMatch>> = BTreeMap::new();

        let domains: Vec<Domain> = Self::list_domains()?
            .into_iter()
            .chain([Domain::Global])
            .collect();

        for domain in domains {
            let loaded = foundation::read_pref_domain(&domain.to_string())?;
            let mut matches = Vec::new();

            Self::find_in_value(&loaded, &word_lower, String::new(), &mut matches);
            if !matches.is_empty() {
                results.insert(domain.to_string(), matches);
            }
        }
        Ok(results)
    }

    /// Recursively searches a plist Value.
    fn find_in_value(
        val: &PrefValue,
        word_lower: &str,
        key_path: String,
        matches: &mut Vec<FindMatch>,
    ) {
        fn contains_word(haystack: &str, needle: &str) -> bool {
            haystack.to_lowercase().contains(needle)
        }
        match val {
            PrefValue::Dictionary(dict) => {
                for (k, v) in dict {
                    let new_key_path = if key_path.is_empty() {
                        k.clone()
                    } else {
                        format!("{key_path}.{k}")
                    };
                    if contains_word(k, word_lower) {
                        matches.push(FindMatch {
                            key: new_key_path.clone(),
                            value: v.clone(),
                        });
                    }
                    Self::find_in_value(v, word_lower, new_key_path, matches);
                }
            }
            PrefValue::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    let new_key_path = format!("{key_path}[{i}]");
                    Self::find_in_value(v, word_lower, new_key_path, matches);
                }
            }
            _ => {
                if contains_word(&val.to_string(), word_lower) {
                    matches.push(FindMatch {
                        key: key_path.clone(),
                        value: val.clone(),
                    });
                }
            }
        }
    }

    /// Read a value from the given domain and key.
    pub fn read(domain: Domain, key: &str) -> Result<PrefValue> {
        let cf_name = &domain.get_cf_name();
        foundation::read_pref(cf_name, key)
    }

    /// Read an entire domain.
    pub fn read_domain(domain: Domain) -> Result<PrefValue> {
        let cf_name = &domain.get_cf_name();
        let mut result = match foundation::read_pref_domain(cf_name)? {
            PrefValue::Dictionary(inner) => inner,
            _ => unreachable!(),
        };

        let matching_domains: Vec<Domain> = Preferences::list_domains()?
            .into_iter()
            .filter(|d| d.to_string().starts_with(&format!("{}.", domain)))
            .collect();

        for m_domain in matching_domains {
            let m_result = match foundation::read_pref_domain(&m_domain.get_cf_name())? {
                PrefValue::Dictionary(inner) => inner,
                _ => unreachable!(),
            };
            result.extend(m_result);
        }

        Ok(PrefValue::Dictionary(result))
    }

    /// Write a value to the given domain and key.
    ///
    /// If the domain file does not exist, it will be created.
    /// If the key already exists, its value will be overwritten.
    pub fn write(domain: Domain, key: &str, value: PrefValue) -> Result<()> {
        let cf_name = &domain.get_cf_name();
        foundation::write_pref(cf_name, key, &value)?;

        Ok(())
    }

    /// Delete a key from the given domain.
    pub fn delete(domain: Domain, key: &str) -> Result<()> {
        let cf_name = &domain.get_cf_name();
        foundation::delete_key(cf_name, key)
    }

    /// Delete a whole domain.
    pub fn delete_domain(domain: Domain) -> Result<()> {
        let cf_name = &domain.get_cf_name();
        foundation::delete_domain(cf_name)
    }

    /// Read the type of a value at the given key in the specified domain.
    ///
    /// Returns a string describing the type.
    pub fn read_type(domain: Domain, key: &str) -> Result<String> {
        let cf_name = domain.get_cf_name();
        let loaded = foundation::read_pref(&cf_name, key)?;

        Ok(loaded.get_type().to_string())
    }

    /// Rename a key in the given domain.
    ///
    /// Moves the value from `old_key` to `new_key` within the domain plist.
    pub fn rename(domain: Domain, old_key: &str, new_key: &str) -> Result<()> {
        let cf_name = &domain.get_cf_name();

        // Read old value
        let val = foundation::read_pref(cf_name, old_key)?;

        foundation::write_pref(cf_name, new_key, &val)?;
        foundation::delete_key(cf_name, old_key)?;

        Ok(())
    }

    /// Import a plist file into the specified domain.
    ///
    /// Replaces any existing file for the domain.
    pub fn import(domain: Domain, import_path: &str) -> Result<()> {
        let data = fs::read(import_path)?;

        let plist_val = Value::from_reader(Cursor::new(&data))?;

        let dict = match plist_val {
            Value::Dictionary(d) => d,
            _ => {
                bail!("Import must be a dictionary at root.")
            }
        };

        let cf_name = &domain.get_cf_name();
        for (k, v) in dict {
            let pv = plist_to_prefvalue(&v);
            foundation::write_pref(cf_name, &k, &pv)?;
        }
        Ok(())
    }

    /// Export a domain's plist file to the specified path.
    pub fn export(domain: Domain, export_path: &str) -> Result<()> {
        let cf_name = &domain.get_cf_name();
        let pref = foundation::read_pref_domain(cf_name)?;

        if !matches!(pref, PrefValue::Dictionary(_)) {
            bail!("CF export produced non-dictionary root")
        }

        let plist = prefvalue_to_plist(&pref);
        let path = PathBuf::from(export_path);

        let file = File::create(path)?;
        plist
            .to_writer_binary(file)
            .context("failed to export CF domain to plist")?;

        Ok(())
    }

    /// Batch-write multiple key–value pairs for domains concurrently.
    ///
    /// # Concurrency & Grouping
    /// - The input is a vector of tuples `(Domain, String, PrefValue)`.
    /// - All write requests are grouped by domain.
    ///
    /// # Behavior
    /// - Only the designated keys are updated in each plist; the entire domain is not replaced.
    /// - For CoreFoundation domains, each key is written individually.
    ///
    /// # Errors
    /// - If any write fails, the operation returns an error.
    pub fn write_batch(batch: Vec<(Domain, String, PrefValue)>) -> Result<()> {
        let mut groups: HashMap<Domain, Vec<(String, PrefValue)>> = HashMap::new();

        // Group write requests by domain.
        for (domain, key, value) in batch {
            groups.entry(domain).or_default().push((key, value));
        }

        for (domain, writes) in groups {
            let cf_name = &domain.get_cf_name();

            for (key, value) in writes {
                foundation::write_pref(cf_name, &key, &value)?;
            }
        }

        Ok(())
    }

    /// Batch-read multiple keys for domains concurrently.
    ///
    /// # Concurrency & Grouping
    /// - The input is a vector of tuples `(Domain, String)`.
    /// - Requests are grouped by domain.
    ///
    /// # Behavior
    /// - The result is a vector of tuples `(Domain, String, ReadResult)`.
    ///
    /// # Errors
    /// - If any read fails (e.g., key not found), the operation returns an error.
    pub fn read_batch(batch: Vec<(Domain, String)>) -> Result<Vec<(Domain, String, PrefValue)>> {
        let mut groups: HashMap<Domain, Vec<String>> = HashMap::new();

        // group requests by domain
        for (domain, key) in batch {
            groups.entry(domain).or_default().push(key);
        }

        let mut results = Vec::new();

        for (domain, keys) in groups {
            let cf_name = &domain.get_cf_name();

            for k in keys {
                let pref_val = foundation::read_pref(cf_name, &k)?;
                results.push((domain.clone(), k.clone(), pref_val));
            }
        }

        Ok(results)
    }

    /// Batch-delete multiple keys for domains concurrently.
    ///
    /// # Concurrency & Grouping
    /// - The input is a vector of tuples `(Domain, String)`.
    /// - Requests are grouped by domain.
    ///
    /// # Behavior
    /// - Only the specified keys are removed from the domain.
    ///
    /// # Errors
    /// - If any deletion fails (e.g., key not found), the operation returns an error.
    pub fn delete_batch(batch: Vec<(Domain, String)>) -> Result<()> {
        let mut groups: HashMap<Domain, Vec<String>> = HashMap::new();

        // group requests by domain
        for (domain, key) in batch {
            groups.entry(domain).or_default().push(key);
        }

        for (domain, keys) in groups {
            let cf_name = &domain.get_cf_name();

            for k in keys {
                foundation::delete_key(cf_name, &k)?
            }
        }

        Ok(())
    }
}
