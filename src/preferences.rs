//! The API side of defaults-rs.

use crate::prefs::error::PrefError;
use crate::prefs::types::{Domain, PrefValue, ReadResult};
use futures::future::join_all;
use plist::Value as PlistValue;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Provides asynchronous operations for reading, writing, deleting, and managing
/// macOS plist preference files in user or global domains.
pub struct Preferences;

/// Result of a find operation.
pub struct FindMatch {
    pub key_path: String,
    pub value: String,
}

impl Preferences {
    /// Search all domains for keys or values containing the given word (case-insensitive).
    ///
    /// Returns a map from domain name to a vector of FindMatch.
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
            let plist = match Self::read(domain, None).await {
                Ok(ReadResult::Plist(plist)) => plist,
                _ => continue,
            };
            let mut matches = Vec::new();
            Self::find_in_value(&plist, &word_lower, String::new(), &mut matches);
            if !matches.is_empty() {
                results.insert(domain_name, matches);
            }
        }
        Ok(results)
    }

    fn find_in_value(
        val: &plist::Value,
        word_lower: &str,
        key_path: String,
        matches: &mut Vec<FindMatch>,
    ) {
        // Helper to check if a string contains the word (case-insensitive)
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
                    // Check key match
                    if contains_word(k, word_lower) {
                        matches.push(FindMatch {
                            key_path: new_key_path.clone(),
                            value: Self::plist_value_to_string(v),
                        });
                    }
                    // Recurse
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

    fn plist_value_to_string(val: &plist::Value) -> String {
        match val {
            plist::Value::String(s) => format!("{:?}", s),
            plist::Value::Integer(i) => i.to_string(),
            plist::Value::Real(f) => f.to_string(),
            plist::Value::Boolean(b) => b.to_string(),
            plist::Value::Array(_) | plist::Value::Dictionary(_) => format!("{:?}", val),
            _ => format!("{:?}", val),
        }
    }
    /// Read a value from the given domain and key.
    ///
    /// If `key` is `None`, returns the entire domain as a `plist::Value`.
    /// If `key` is provided, returns the value at that key as a `PrefValue`.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to read from (global or user).
    /// * `key` - The key to read, or `None` to read the entire domain.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be read,
    /// `PrefError::Other` if the plist cannot be parsed,
    /// `PrefError::KeyNotFound` if the key does not exist,
    /// or `PrefError::InvalidType` if the plist is not a dictionary.
    pub async fn read(domain: Domain, key: Option<&str>) -> Result<ReadResult, PrefError> {
        let path = Self::domain_path(&domain);
        let mut file = File::open(&path).await.map_err(PrefError::Io)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
        let plist = PlistValue::from_reader_xml(Cursor::new(&buf[..]))
            .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
            .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?;

        fn convert(val: &PlistValue) -> PrefValue {
            match val {
                PlistValue::String(s) => PrefValue::String(s.clone()),
                PlistValue::Integer(i) => PrefValue::Integer(i.as_signed().unwrap()),
                PlistValue::Real(f) => PrefValue::Float(*f),
                PlistValue::Boolean(b) => PrefValue::Boolean(*b),
                PlistValue::Array(arr) => PrefValue::Array(arr.iter().map(convert).collect()),
                PlistValue::Dictionary(dict) => PrefValue::Dictionary(
                    dict.iter().map(|(k, v)| (k.clone(), convert(v))).collect(),
                ),
                _ => PrefValue::String(format!("{:?}", val)),
            }
        }

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
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to write to (global or user).
    /// * `key` - The key to write.
    /// * `value` - The value to write.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be read or written,
    /// `PrefError::Other` if the plist cannot be parsed or written,
    /// or `PrefError::InvalidType` if the root plist is not a dictionary.
    pub async fn write(domain: Domain, key: &str, value: PrefValue) -> Result<(), PrefError> {
        let path = Self::domain_path(&domain);

        let mut root = if fs::metadata(&path).await.is_ok() {
            let mut file = File::open(&path).await.map_err(PrefError::Io)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
            PlistValue::from_reader_xml(Cursor::new(&buf[..]))
                .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
                .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?
        } else {
            PlistValue::Dictionary(plist::Dictionary::new())
        };

        if let PlistValue::Dictionary(ref mut dict) = root {
            dict.insert(key.to_string(), Self::to_plist_value(&value));
        } else {
            return Err(PrefError::InvalidType);
        }

        let mut buf = Vec::new();
        root.to_writer_xml(&mut buf)
            .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;

        let mut file = File::create(&path).await.map_err(PrefError::Io)?;
        file.write_all(&buf).await.map_err(PrefError::Io)?;
        file.flush().await.map_err(PrefError::Io)?;
        Ok(())
    }

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
            let mut root = if fs::metadata(&path).await.is_ok() {
                let mut file = File::open(&path).await.map_err(PrefError::Io)?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
                PlistValue::from_reader_xml(Cursor::new(&buf[..]))
                    .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
                    .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?
            } else {
                PlistValue::Dictionary(plist::Dictionary::new())
            };

            if let PlistValue::Dictionary(ref mut dict) = root {
                for (key, value) in writes {
                    dict.insert(key, Self::to_plist_value(&value));
                }
            } else {
                return Err(PrefError::InvalidType);
            }

            let mut buf = Vec::new();
            root.to_writer_xml(&mut buf)
                .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;
            let mut file = File::create(&path).await.map_err(PrefError::Io)?;
            file.write_all(&buf).await.map_err(PrefError::Io)?;
            file.flush().await.map_err(PrefError::Io)?;
            Ok(())
        });
        let results = join_all(tasks).await;
        for res in results {
            res?;
        }
        Ok(())
    }

    /// Delete a key from the given domain.
    ///
    /// If `key` is `None`, deletes the entire domain file.
    /// If `key` is provided, removes the key from the domain plist.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to delete from (global or user).
    /// * `key` - The key to delete, or `None` to delete the entire domain file.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be read or written,
    /// `PrefError::Other` if the plist cannot be parsed or written,
    /// `PrefError::KeyNotFound` if the key or file does not exist,
    /// or `PrefError::InvalidType` if the root plist is not a dictionary.
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
                if fs::metadata(&path).await.is_err() {
                    return Err(PrefError::KeyNotFound);
                }
                let mut file = File::open(&path).await.map_err(PrefError::Io)?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
                let mut plist = PlistValue::from_reader_xml(Cursor::new(&buf[..]))
                    .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
                    .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?;

                if let PlistValue::Dictionary(ref mut dict) = plist {
                    if dict.remove(k).is_some() {
                        let mut out_buf = Vec::new();
                        plist
                            .to_writer_xml(&mut out_buf)
                            .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;
                        let mut file = File::create(&path).await.map_err(PrefError::Io)?;
                        file.write_all(&out_buf).await.map_err(PrefError::Io)?;
                        file.flush().await.map_err(PrefError::Io)?;
                        Ok(())
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
    /// Returns a string describing the type: "string", "integer", "float", "boolean", "array", "dictionary", or "unknown".
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to read from.
    /// * `key` - The key whose type to check.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be read,
    /// `PrefError::Other` if the plist cannot be parsed,
    /// `PrefError::KeyNotFound` if the key does not exist,
    /// or `PrefError::InvalidType` if the root plist is not a dictionary.
    pub async fn read_type(domain: Domain, key: &str) -> Result<String, PrefError> {
        let path = Self::domain_path(&domain);
        let mut file = File::open(&path).await.map_err(PrefError::Io)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
        let plist = PlistValue::from_reader_xml(Cursor::new(&buf[..]))
            .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
            .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?;

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
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to operate on.
    /// * `old_key` - The existing key to rename.
    /// * `new_key` - The new key name.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be read or written,
    /// `PrefError::Other` if the plist cannot be parsed or written,
    /// `PrefError::KeyNotFound` if the old key does not exist,
    /// or `PrefError::InvalidType` if the root plist is not a dictionary.
    pub async fn rename(domain: Domain, old_key: &str, new_key: &str) -> Result<(), PrefError> {
        let path = Self::domain_path(&domain);
        if fs::metadata(&path).await.is_err() {
            return Err(PrefError::KeyNotFound);
        }
        let mut file = File::open(&path).await.map_err(PrefError::Io)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;
        let mut plist = PlistValue::from_reader_xml(Cursor::new(&buf[..]))
            .or_else(|_| PlistValue::from_reader(Cursor::new(&buf[..])))
            .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?;

        if let PlistValue::Dictionary(ref mut dict) = plist {
            if let Some(val) = dict.remove(old_key) {
                dict.insert(new_key.to_string(), val);
                let mut out_buf = Vec::new();
                plist
                    .to_writer_xml(&mut out_buf)
                    .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;
                let mut file = File::create(&path).await.map_err(PrefError::Io)?;
                file.write_all(&out_buf).await.map_err(PrefError::Io)?;
                file.flush().await.map_err(PrefError::Io)?;
                Ok(())
            } else {
                Err(PrefError::KeyNotFound)
            }
        } else {
            Err(PrefError::InvalidType)
        }
    }

    /// Import a plist file into the specified domain.
    ///
    /// Copies the file at `import_path` to the domain's plist location, replacing any existing file.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to import into.
    /// * `import_path` - The path to the source plist file.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be copied.
    pub async fn import(domain: Domain, import_path: &str) -> Result<(), PrefError> {
        let dest_path = Self::domain_path(&domain);
        fs::copy(import_path, dest_path)
            .await
            .map_err(PrefError::Io)?;
        Ok(())
    }

    /// Export a domain's plist file to the specified path.
    ///
    /// Copies the domain's plist file to `export_path`.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to export.
    /// * `export_path` - The destination path for the exported plist file.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the file cannot be copied.
    pub async fn export(domain: Domain, export_path: &str) -> Result<(), PrefError> {
        let src_path = Self::domain_path(&domain);
        fs::copy(src_path, export_path)
            .await
            .map_err(PrefError::Io)?;
        Ok(())
    }

    /// Get the filesystem path for a given domain.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to get the path for.
    ///
    /// # Returns
    ///
    /// Returns a `PathBuf` pointing to the domain's plist file.
    ///
    /// # Panics
    ///
    /// Panics if the `HOME` environment variable is not set.
    pub(crate) fn domain_path(domain: &Domain) -> PathBuf {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        match domain {
            Domain::Global => PathBuf::from(format!(
                "{}/Library/Preferences/.GlobalPreferences.plist",
                home
            )),
            Domain::User(name) => {
                PathBuf::from(format!("{}/Library/Preferences/{}.plist", home, name))
            }
        }
    }

    /// Pretty-print a `PlistValue` in Apple-style format (for CLI).
    ///
    /// # Arguments
    ///
    /// * `val` - The plist value to print.
    /// * `indent` - The indentation level (number of indents).
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

    /// List all available domains in `~/Library/Preferences`.
    ///
    /// Returns a sorted vector of domain names, with "NSGlobalDomain" for the global domain.
    ///
    /// # Errors
    ///
    /// Returns `PrefError::Io` if the directory cannot be read,
    /// or `PrefError::Other` if the `HOME` environment variable is not set.
    pub async fn list_domains() -> Result<Vec<String>, PrefError> {
        let home =
            std::env::var("HOME").map_err(|e| PrefError::Other(format!("HOME env error: {e}")))?;
        let prefs_dir = PathBuf::from(format!("{}/Library/Preferences", home));
        let mut entries = match fs::read_dir(&prefs_dir).await {
            Ok(rd) => rd,
            Err(e) => return Err(PrefError::Io(e)),
        };

        let mut domains = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(PrefError::Io)? {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                if fname == ".GlobalPreferences.plist" {
                    domains.push("NSGlobalDomain".to_string());
                } else if fname.ends_with(".plist") && !fname.starts_with('.') {
                    let dom = fname.trim_end_matches(".plist").to_string();
                    domains.push(dom);
                }
            }
        }
        domains.sort();
        Ok(domains)
    }

    /// Quote a key for Apple-style plist output if necessary.
    ///
    /// Keys containing only alphanumeric characters, '-' or '_' are not quoted.
    /// Otherwise, the key is quoted and internal quotes are escaped.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to quote.
    ///
    /// # Returns
    ///
    /// Returns the quoted or unquoted key as a `String`.
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

    /// Quote a string for Apple-style plist output if necessary.
    ///
    /// Strings containing only alphanumeric characters, '-' or '_' are not quoted.
    /// Otherwise, the string is quoted and internal quotes are escaped.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to quote.
    ///
    /// # Returns
    ///
    /// Returns the quoted or unquoted string as a `String`.
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
