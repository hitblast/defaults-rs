use crate::prefs::{
    error::PrefError,
    types::{Domain, FindMatch, LoadedPlist, PrefValue, ReadResult},
};
use futures::future::join_all;
use libc::{gid_t, uid_t};
use once_cell::sync::Lazy;
use plist::Value as PlistValue;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::CString,
    fs::{Permissions, metadata, set_permissions},
    io::Cursor,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt, fs::PermissionsExt},
    },
    path::PathBuf,
};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
};

/// Cached HOME environment variable.
static HOME: Lazy<String> =
    Lazy::new(|| std::env::var("HOME").expect("HOME environment variable not set"));

/// Provides asynchronous operations for reading, writing, deleting, and managing
/// macOS plist preference files in user or global domains.
pub struct Preferences;

impl Preferences {
    /// loads the plist for the specified path.
    async fn load_plist(path: &PathBuf) -> Result<LoadedPlist, PrefError> {
        let metadata = fs::metadata(path).await;
        let orig_owner = metadata.as_ref().ok().map(|m| (m.uid(), m.gid()));

        let (plist, is_binary) = if metadata.is_ok() {
            let mut file = File::open(path).await.map_err(PrefError::Io)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;

            // try to parse as XML first, and if that fails, fallback to another format
            if let Ok(plist) = PlistValue::from_reader_xml(Cursor::new(&buf[..])) {
                (plist, false)
            } else {
                let plist = PlistValue::from_reader(Cursor::new(&buf[..]))
                    .map_err(|e| PrefError::Other(format!("Plist parse error: {}", e)))?;
                (plist, true)
            }
        } else {
            // file does not exist, return a new empty dictionary value
            (PlistValue::Dictionary(plist::Dictionary::new()), false)
        };

        Ok(LoadedPlist {
            plist,
            orig_owner,
            is_binary,
        })
    }

    /// Saves the plist to the specified path and restores ownership using an atomic write.
    async fn save_plist(
        path: &PathBuf,
        plist: &PlistValue,
        orig_owner: Option<(u32, u32)>,
        is_binary: bool,
    ) -> Result<(), PrefError> {
        // acquire an exclusive lock on the file if possible
        let lock_file = std::fs::OpenOptions::new().read(true).open(path);
        let mut guard_fd = None;
        if let Ok(file) = lock_file {
            let fd = file.as_raw_fd();
            // safety: use `flock` to ensure exclusive access to the file while writing
            unsafe {
                libc::flock(fd, libc::LOCK_EX);
            }
            guard_fd = Some(file);
        }

        // prepare the buffer by writing the plist in the requested format
        let mut buf = Vec::new();
        if is_binary {
            plist
                .to_writer_binary(&mut buf)
                .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;
        } else {
            plist
                .to_writer_xml(&mut buf)
                .map_err(|e| PrefError::Other(format!("Plist write error: {}", e)))?;
        }

        // capture original file permissions
        let orig_perm = fs::metadata(path).await.ok().map(|m| m.permissions());

        // retrieve directory and file_name from the path
        let dir = path
            .parent()
            .ok_or_else(|| PrefError::Other("Invalid path: no parent directory".into()))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| PrefError::Other("Invalid path: no file name".into()))?;

        // create the temporary file path
        let tmp_file_name = format!("{}.tmp", file_name.to_string_lossy());
        let tmp_path = dir.join(tmp_file_name);

        // write the buffer to a temporary file
        let mut tmp_file = File::create(&tmp_path).await.map_err(PrefError::Io)?;
        tmp_file.write_all(&buf).await.map_err(PrefError::Io)?;
        tmp_file.flush().await.map_err(PrefError::Io)?;

        // restore ownership on the temporary file, if required
        if let Some((uid, gid)) = orig_owner {
            let _ = Self::restore_ownership(&tmp_path, uid, gid);
        }

        // atomically replace the original file with the temporary file
        tokio::fs::rename(&tmp_path, path)
            .await
            .map_err(PrefError::Io)?;

        // restore the original permissions if they were captured
        if let Some(perm) = orig_perm {
            std::fs::set_permissions(path, perm).map_err(PrefError::Io)?;
        }

        // release the file lock if acquired
        if let Some(file) = guard_fd {
            let fd = file.as_raw_fd();
            // safety: Matching the previous flock, we unlock the file
            unsafe {
                libc::flock(fd, libc::LOCK_UN);
            }
        }

        Ok(())
    }

    /// Restores file ownership to the given uid/gid.
    fn restore_ownership<P: AsRef<std::path::Path>>(
        path: P,
        uid: u32,
        gid: u32,
    ) -> Result<(), PrefError> {
        // obtain a reference to the path and retrieve its metadata (including permissions)
        let path_ref = path.as_ref();
        let meta = metadata(path_ref).map_err(PrefError::Io)?;
        let orig_mode = meta.permissions().mode();

        // convert the file path to a null-terminated C string
        let c_path = CString::new(path_ref.as_os_str().as_bytes())
            .map_err(|_| PrefError::Other("Invalid file path with interior null byte".into()))?;

        // safety: call libc::chown with a valid C string to change ownership
        let res = unsafe { libc::chown(c_path.as_ptr(), uid as uid_t, gid as gid_t) };
        if res == 0 {
            // restore the original file mode (permissions)
            set_permissions(path_ref, Permissions::from_mode(orig_mode)).map_err(PrefError::Io)?;
            Ok(())
        } else {
            Err(PrefError::Io(std::io::Error::last_os_error()))
        }
    }

    /// Search all domains for keys or values containing the given word (case-insensitive).
    pub async fn find(word: &str) -> Result<BTreeMap<String, Vec<FindMatch>>, PrefError> {
        let word_lower = word.to_lowercase();
        let mut results: BTreeMap<String, Vec<FindMatch>> = BTreeMap::new();

        let domains = Self::list_domains().await?;

        for domain_name in domains {
            let domain = if domain_name == "NSGlobalDomain" {
                Domain::Global
            } else {
                Domain::User(domain_name.clone())
            };

            let loaded = match Self::read_internal(&domain).await {
                Ok(l) => l,
                Err(_) => continue,
            };

            let plist = loaded.plist;
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
    async fn read_internal(domain: &Domain) -> Result<LoadedPlist, PrefError> {
        let path = Self::domain_path(domain);
        Self::load_plist(&path).await
    }

    /// Read a value from the given domain and key.
    ///
    /// If `key` is `None`, returns the entire domain as a `plist::Value`.
    /// If `key` is provided, returns the value at that key as a `PrefValue`.
    pub async fn read(domain: Domain, key: Option<&str>) -> Result<ReadResult, PrefError> {
        let loaded = Self::read_internal(&domain).await?;
        let plist = loaded.plist;

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
        let mut loaded = Self::load_plist(&path).await?;
        if let PlistValue::Dictionary(ref mut dict) = loaded.plist {
            dict.insert(key.to_string(), Self::to_plist_value(&value));
        } else {
            return Err(PrefError::InvalidType);
        }
        Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary).await
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
                let mut loaded = Self::load_plist(&path).await?;
                if let PlistValue::Dictionary(ref mut dict) = loaded.plist {
                    if dict.remove(k).is_some() {
                        Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary)
                            .await
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
        let loaded = Self::read_internal(&domain).await?;
        let plist = loaded.plist;
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
        let mut loaded = Self::load_plist(&path).await?;
        if let PlistValue::Dictionary(ref mut dict) = loaded.plist {
            if let Some(val) = dict.remove(old_key) {
                dict.insert(new_key.to_string(), val);
                Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary).await
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
        let orig_owner = fs::metadata(&dest_path)
            .await
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
            Domain::Path(path) => path.clone(),
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

    /// Batch-write multiple key–value pairs for domains concurrently.
    ///
    /// The input is a vector of tuples `(Domain, String, PrefValue)`. For each unique domain,
    /// the write requests are merged and applied in a single I/O operation. This function updates only
    /// the designated keys in the plist, rather than replacing the entire domain.
    pub async fn write_batch(batch: Vec<(Domain, String, PrefValue)>) -> Result<(), PrefError> {
        let mut groups: HashMap<Domain, Vec<(String, PrefValue)>> = HashMap::new();
        // Group write requests by domain.
        for (domain, key, value) in batch {
            groups.entry(domain).or_default().push((key, value));
        }
        let tasks = groups.into_iter().map(|(domain, writes)| async move {
            let path = Self::domain_path(&domain);
            let mut loaded = Self::load_plist(&path).await?;
            if let PlistValue::Dictionary(ref mut dict) = loaded.plist {
                for (key, value) in writes {
                    dict.insert(key, Self::to_plist_value(&value));
                }
            } else {
                return Err(PrefError::InvalidType);
            }
            Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary).await
        });
        for res in join_all(tasks).await {
            res?;
        }
        Ok(())
    }

    /// Batch-read multiple keys for domains concurrently.
    ///
    /// For each unique domain in the provided batch, this method reads the entire plist concurrently
    /// and then extracts the requested key(s). If a request specifies `None` for the key,
    /// the entire domain plist is returned; otherwise, the key's value is returned.
    ///
    /// Returns a vector of tuples `(Domain, Option<String>, ReadResult)` where the:
    /// - `Domain` is the domain read,
    /// - `Option<String>` is the key used (or `None` for the entire domain),
    /// - `ReadResult` is either the value at that key or the full plist.
    #[allow(clippy::type_complexity)]
    pub async fn read_batch(
        batch: Vec<(Domain, Option<String>)>,
    ) -> Result<Vec<(Domain, Option<String>, ReadResult)>, PrefError> {
        let mut groups: HashMap<Domain, Vec<Option<String>>> = HashMap::new();

        // group requests by domain
        for (domain, key) in batch {
            groups.entry(domain).or_default().push(key);
        }

        // spawn concurrent futures to process each domain
        let futures = groups.into_iter().map(|(domain, keys)| async move {
            let loaded = Self::read_internal(&domain).await?;
            let plist = loaded.plist;

            // process each key for the current domain
            let results: Result<Vec<_>, PrefError> = keys
                .into_iter()
                .map(|opt_key| {
                    match opt_key.as_deref() {
                        None => Ok((domain.clone(), None, ReadResult::Plist(plist.clone()))),
                        Some(k) => {
                            // ensure the plist is a dictionary
                            let dict = match plist {
                                PlistValue::Dictionary(ref d) => d,
                                _ => return Err(PrefError::InvalidType),
                            };
                            // look up the value and return an error if not found
                            let val = dict.get(k).ok_or(PrefError::KeyNotFound)?;
                            Ok((
                                domain.clone(),
                                Some(k.to_string()),
                                ReadResult::Value(convert(val)),
                            ))
                        }
                    }
                })
                .collect();

            results
        });

        // execute all domain reads concurrently
        let grouped_results: Result<Vec<Vec<(Domain, Option<String>, ReadResult)>>, _> =
            futures::future::join_all(futures)
                .await
                .into_iter()
                .collect();
        Ok(grouped_results?.into_iter().flatten().collect())
    }
    /// Batch-delete multiple keys for domains concurrently.
    ///
    /// For each unique domain in the provided batch, this method loads the plist concurrently.
    /// If any request for a domain has a key of `None`, the entire domain file is deleted.
    /// Otherwise, the specified keys are removed from the domain.
    pub async fn delete_batch(batch: Vec<(Domain, Option<String>)>) -> Result<(), PrefError> {
        let mut groups: HashMap<Domain, Vec<Option<String>>> = HashMap::new();

        // group requests by domain
        for (domain, key) in batch {
            groups.entry(domain).or_default().push(key);
        }

        // spawn concurrent futures to process each domain deletion
        let futures = groups.into_iter().map(|(domain, keys)| async move {
            if keys.iter().any(|k| k.is_none()) {
                // if any key is None, delete the entire domain
                Self::delete(domain.clone(), None).await
            } else {
                let path = Self::domain_path(&domain);
                let mut loaded = Self::load_plist(&path).await?;
                if let PlistValue::Dictionary(ref mut dict) = loaded.plist {
                    for k in keys.into_iter().flatten() {
                        if dict.remove(&k).is_none() {
                            return Err(PrefError::KeyNotFound);
                        }
                    }
                } else {
                    return Err(PrefError::InvalidType);
                }
                Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary).await
            }
        });

        // execute all deletions concurrently
        futures::future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
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
