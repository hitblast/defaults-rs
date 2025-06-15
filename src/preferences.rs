//! The API side of defaults-rs.

use crate::prefs::error::PrefError;
use crate::prefs::types::{Domain, FindMatch, PrefValue, ReadResult};
use futures::future::join_all;
use once_cell::sync::Lazy;
use plist::Value as PlistValue;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use libc::{gid_t, uid_t};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

/// Cached HOME environment variable.
static HOME: Lazy<String> =
    Lazy::new(|| std::env::var("HOME").expect("HOME environment variable not set"));

/// Provides asynchronous operations for reading, writing, deleting, and managing
/// macOS plist preference files in user or global domains.
pub struct Preferences;

impl Preferences {
    /// Loads the plist for the specified path.
    async fn load_plist(path: &PathBuf) -> Result<(PlistValue, Option<(u32, u32)>), PrefError> {
        let orig_owner = std::fs::metadata(path).ok().map(|m| (m.uid(), m.gid()));

        let plist = if fs::metadata(path).await.is_ok() {
            let mut file = File::open(path).await.map_err(PrefError::Io)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
            PlistValue::from_reader_xml(Cursor::new(&buf[..]))
                .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
                .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?
        } else {
            PlistValue::Dictionary(plist::Dictionary::new())
        };

        Ok((plist, orig_owner))
    }

    /// Saves the plist to the specified path and restores ownership.
    async fn save_plist(
        path: &PathBuf,
        plist: &PlistValue,
        orig_owner: Option<(u32, u32)>,
    ) -> Result<(), PrefError> {
        let mut buf = Vec::new();
        plist
            .to_writer_xml(&mut buf)
            .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;
        let mut file = File::create(path).await.map_err(PrefError::Io)?;
        file.write_all(&buf).await.map_err(PrefError::Io)?;
        file.flush().await.map_err(PrefError::Io)?;
        if let Some((uid, gid)) = orig_owner {
            let _ = Self::restore_ownership(path, uid, gid);
        }
        Ok(())
    }

    /// Restores file ownership to the given uid/gid.
    fn restore_ownership<P: AsRef<std::path::Path>>(
        path: P,
        uid: u32,
        gid: u32,
    ) -> std::io::Result<()> {
        use std::ffi::CString;
        let c_path = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();

        // safety: chown is a syscall, returns 0 on success, -1 on error
        let res = unsafe { libc::chown(c_path.as_ptr(), uid as uid_t, gid as gid_t) };
        if res == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    /// Search all domains for keys or values containing the given word (case-insensitive).
    pub async fn find(
        word: &str,
    ) -> Result<std::collections::BTreeMap<String, Vec<FindMatch>>, PrefError> {
        use std::collections::BTreeMap;
        let word_lower = word.to_lowercase();
        let mut results: BTreeMap<String, Vec<FindMatch>> = BTreeMap::new();
        let domains = Self::list_domains().await?;
        for domain_name in domains {
            let domain = if domain_name == "NSGlobalDomain" {
                Domain::Global
            } else {
                Domain::User(domain_name.clone())
            };
            let (plist, _) = match Self::read_internal(&domain).await {
                Ok((p, o)) => (p, o),
                Err(_) => continue,
            };
            let mut matches = Vec::new();
            Self::find_in_value(&plist, &word_lower, String::new(), &mut matches);
            if !matches.is_empty() {
                results.insert(domain_name, matches);
            }
        }
        Ok(results)
    }

    /// Recursively searches a plist Value.
    fn find_in_value(
        val: &plist::Value,
        word_lower: &str,
        key_path: String,
        matches: &mut Vec<FindMatch>,
    ) {
        fn contains_word(haystack: &str, needle: &str) -> bool {
            haystack.to_lowercase().contains(needle)
        }
        match val {
            plist::Value::Dictionary(dict) => {
                for (k, v) in dict {
                    let new_key_path = if key_path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", key_path, k)
                    };
                    if contains_word(k, word_lower) {
                        matches.push(FindMatch {
                            key_path: new_key_path.clone(),
                            value: Self::plist_value_to_string(v),
                        });
                    }
                    Self::find_in_value(v, word_lower, new_key_path, matches);
                }
            }
            plist::Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    let new_key_path = format!("{}[{}]", key_path, i);
                    Self::find_in_value(v, word_lower, new_key_path, matches);
                }
            }
            _ => {
                let val_str = Self::plist_value_to_string(val);
                if contains_word(&val_str, word_lower) {
                    matches.push(FindMatch {
                        key_path,
                        value: val_str,
                    });
                }
            }
        }
    }

    /// Converts a plist Value into a string.
    fn plist_value_to_string(val: &plist::Value) -> String {
        match val {
            plist::Value::String(s) => format!("{:?}", s),
            plist::Value::Integer(i) => i.as_signed().unwrap_or(0).to_string(),
            plist::Value::Real(f) => f.to_string(),
            plist::Value::Boolean(b) => b.to_string(),
            plist::Value::Array(_) | plist::Value::Dictionary(_) => format!("{:?}", val),
            _ => format!("{:?}", val),
        }
    }

    /// Loads the plist for the given domain.
    async fn read_internal(domain: &Domain) -> Result<(PlistValue, Option<(u32, u32)>), PrefError> {
        let path = Self::domain_path(domain);
        Self::load_plist(&path).await
    }

    /// Read a value from the given domain and key.
    ///
    /// If `key` is `None`, returns the entire domain as a `plist::Value`.
    /// If `key` is provided, returns the value at that key as a `PrefValue`.
    pub async fn read(domain: Domain, key: Option<&str>) -> Result<ReadResult, PrefError> {
        let (plist, _) = Self::read_internal(&domain).await?;
        match key {
            Some(k) => {
                if let PlistValue::Dictionary(dict) = &plist {
                    match dict.get(k) {
                        Some(val) => Ok(ReadResult::Value(convert(val))),
                        None => Err(PrefError::KeyNotFound),
                    }
                } else {
                    Err(PrefError::InvalidType)
                }
            }
            None => Ok(ReadResult::Plist(plist)),
        }
    }

    /// Write a value to the given domain and key.
    ///
    /// If the domain file does not exist, it will be created.
    /// If the key already exists, its value will be overwritten.
    pub async fn write(domain: Domain, key: &str, value: PrefValue) -> Result<(), PrefError> {
        let path = Self::domain_path(&domain);
        let (mut plist, orig_owner) = Self::load_plist(&path).await?;
        if let PlistValue::Dictionary(ref mut dict) = plist {
            dict.insert(key.to_string(), Self::to_plist_value(&value));
        } else {
            return Err(PrefError::InvalidType);
        }
        Self::save_plist(&path, &plist, orig_owner).await
    }

    /// Converts a `PrefValue` into a plist Value.
    fn to_plist_value(val: &PrefValue) -> PlistValue {
        match val {
            PrefValue::String(s) => PlistValue::String(s.clone()),
            PrefValue::Integer(i) => PlistValue::Integer((*i).into()),
            PrefValue::Float(f) => PlistValue::Real(*f),
            PrefValue::Boolean(b) => PlistValue::Boolean(*b),
            PrefValue::Array(arr) => {
                PlistValue::Array(arr.iter().map(Self::to_plist_value).collect())
            }
            PrefValue::Dictionary(dict) => PlistValue::Dictionary(
                dict.iter()
                    .map(|(k, v)| (k.clone(), Self::to_plist_value(v)))
                    .collect(),
            ),
        }
    }

    /// Batch write multiple keys for domains concurrently.
    pub async fn write_batch(
        batch: Vec<(Domain, Vec<(String, PrefValue)>)>,
    ) -> Result<(), PrefError> {
        let tasks = batch.into_iter().map(|(domain, writes)| async move {
            let path = Self::domain_path(&domain);
            let (mut plist, orig_owner) = Self::load_plist(&path).await?;
            if let PlistValue::Dictionary(ref mut dict) = plist {
                for (key, value) in writes {
                    dict.insert(key, Self::to_plist_value(&value));
                }
            } else {
                return Err(PrefError::InvalidType);
            }
            Self::save_plist(&path, &plist, orig_owner).await
        });
        for res in join_all(tasks).await {
            res?;
        }
        Ok(())
    }

    /// Delete a key from the given domain.
    ///
    /// If `key` is `None`, deletes the entire domain file.
    /// If `key` is provided, removes the key from the domain plist.
    pub async fn delete(domain: Domain, key: Option<&str>) -> Result<(), PrefError> {
        let path = Self::domain_path(&domain);
        match key {
            None => {
                if fs::metadata(&path).await.is_ok() {
                    fs::remove_file(&path).await.map_err(PrefError::Io)?;
                }
                Ok(())
            }
            Some(k) => {
                let (mut plist, orig_owner) = Self::load_plist(&path).await?;
                if let PlistValue::Dictionary(ref mut dict) = plist {
                    if dict.remove(k).is_some() {
                        Self::save_plist(&path, &plist, orig_owner).await
                    } else {
                        Err(PrefError::KeyNotFound)
                    }
                } else {
                    Err(PrefError::InvalidType)
                }
            }
        }
    }

    /// Read the type of a value at the given key in the specified domain.
    ///
    /// Returns a string describing the type.
    pub async fn read_type(domain: Domain, key: &str) -> Result<String, PrefError> {
        let (plist, _) = Self::read_internal(&domain).await?;
        if let PlistValue::Dictionary(dict) = &plist {
            match dict.get(key) {
                Some(val) => Ok(match val {
                    PlistValue::String(_) => "string",
                    PlistValue::Integer(_) => "integer",
                    PlistValue::Real(_) => "float",
                    PlistValue::Boolean(_) => "boolean",
                    PlistValue::Array(_) => "array",
                    PlistValue::Dictionary(_) => "dictionary",
                    _ => "unknown",
                }
                .to_string()),
                None => Err(PrefError::KeyNotFound),
            }
        } else {
            Err(PrefError::InvalidType)
        }
    }

    /// Rename a key in the given domain.
    ///
    /// Moves the value from `old_key` to `new_key` within the domain plist.
    pub async fn rename(domain: Domain, old_key: &str, new_key: &str) -> Result<(), PrefError> {
        let path = Self::domain_path(&domain);
        let (mut plist, orig_owner) = Self::load_plist(&path).await?;
        if let PlistValue::Dictionary(ref mut dict) = plist {
            if let Some(val) = dict.remove(old_key) {
                dict.insert(new_key.to_string(), val);
                Self::save_plist(&path, &plist, orig_owner).await
            } else {
                Err(PrefError::KeyNotFound)
            }
        } else {
            Err(PrefError::InvalidType)
        }
    }

    /// Import a plist file into the specified domain.
    ///
    /// Replaces any existing file for the domain.
    pub async fn import(domain: Domain, import_path: &str) -> Result<(), PrefError> {
        let dest_path = Self::domain_path(&domain);
        let orig_owner = std::fs::metadata(&dest_path)
            .ok()
            .map(|m| (m.uid(), m.gid()));
        fs::copy(import_path, &dest_path)
            .await
            .map_err(PrefError::Io)?;
        if let Some((uid, gid)) = orig_owner {
            let _ = Self::restore_ownership(&dest_path, uid, gid);
        }
        Ok(())
    }

    /// Export a domain's plist file to the specified path.
    pub async fn export(domain: Domain, export_path: &str) -> Result<(), PrefError> {
        let src_path = Self::domain_path(&domain);
        fs::copy(src_path, export_path)
            .await
            .map_err(PrefError::Io)?;
        Ok(())
    }

    /// Returns the filesystem path for a given domain.
    pub(crate) fn domain_path(domain: &Domain) -> PathBuf {
        match domain {
            Domain::Global => PathBuf::from(format!(
                "{}/Library/Preferences/.GlobalPreferences.plist",
                &*HOME
            )),
            Domain::User(name) => {
                PathBuf::from(format!("{}/Library/Preferences/{}.plist", &*HOME, name))
            }
        }
    }

    /// Pretty-print a `PlistValue` in Apple-style format (for CLI).
    pub fn print_apple_style(val: &plist::Value, indent: usize) {
        let ind = |n| "    ".repeat(n);
        match val {
            plist::Value::Dictionary(dict) => {
                println!("{{");
                for (k, v) in dict {
                    print!("{}{} = ", ind(indent + 1), Self::quote_key(k));
                    Self::print_apple_style(v, indent + 1);
                    println!(";");
                }
                print!("{}}}", ind(indent));
            }
            plist::Value::Array(arr) => {
                println!("(");
                for v in arr {
                    print!("{}", ind(indent + 1));
                    Self::print_apple_style(v, indent + 1);
                    println!(",");
                }
                print!("{})", ind(indent));
            }
            plist::Value::String(s) => print!("{}", Self::quote_string(s)),
            plist::Value::Integer(i) => print!("{}", i),
            plist::Value::Real(f) => print!("{}", f),
            plist::Value::Boolean(b) => print!("{}", if *b { "1" } else { "0" }),
            _ => print!("{:?}", val),
        }
    }

    /// List all available domains in ~/Library/Preferences.
    pub async fn list_domains() -> Result<Vec<String>, PrefError> {
        let home = &*HOME;
        let prefs_dir = PathBuf::from(format!("{}/Library/Preferences", home));
        let mut entries = fs::read_dir(&prefs_dir).await.map_err(PrefError::Io)?;
        let mut domains = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(PrefError::Io)? {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                if fname == ".GlobalPreferences.plist" {
                    domains.push("NSGlobalDomain".to_string());
                } else if fname.ends_with(".plist") && !fname.starts_with('.') {
                    domains.push(fname.trim_end_matches(".plist").to_string());
                }
            }
        }
        domains.sort();
        Ok(domains)
    }

    /// Quotes a key for Apple-style output.
    fn quote_key(key: &str) -> String {
        if key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            key.to_string()
        } else {
            format!("\"{}\"", key.replace('"', "\\\""))
        }
    }

    /// Quotes a string for Apple-style output.
    fn quote_string(s: &str) -> String {
        if s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\\\""))
        }
    }
}

/// Helper to convert a plist Value into a PrefValue.
fn convert(val: &PlistValue) -> PrefValue {
    match val {
        PlistValue::String(s) => PrefValue::String(s.clone()),
        PlistValue::Integer(i) => PrefValue::Integer(i.as_signed().unwrap_or(0)),
        PlistValue::Real(f) => PrefValue::Float(*f),
        PlistValue::Boolean(b) => PrefValue::Boolean(*b),
        PlistValue::Array(arr) => PrefValue::Array(arr.iter().map(convert).collect()),
        PlistValue::Dictionary(dict) => {
            PrefValue::Dictionary(dict.iter().map(|(k, v)| (k.clone(), convert(v))).collect())
        }
        _ => PrefValue::String(format!("{:?}", val)),
    }
}
