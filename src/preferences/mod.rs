// SPDX-License-Identifier: MIT

//! Preferences API for defaults-rs.
//!
//! This module implements all business logic for reading, writing, deleting, importing/exporting,
//! batch operations, and pretty-printing macOS preferences (plist files).
//!
//! It acts as the main interface between the CLI/library and the backend (CoreFoundation or file-based).
//! No CLI parsing or user interaction is performed here; all logic is asynchronous and platform-agnostic.

mod convert;

use std::{
    collections::{BTreeMap, HashMap},
    ffi::CString,
    fs::Permissions,
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, PermissionsExt},
        },
    },
    path::{Path, PathBuf},
};

use crate::{
    core::{
        error::PrefError,
        types::{Domain, FindMatch, LoadedPlist, PrefValue},
    },
    preferences::convert::{plist_to_prefvalue, prefvalue_to_plist},
};
use futures::future::join_all;
use plist::Value;
use tokio::{
    fs::{self, File, set_permissions},
    io::{AsyncReadExt, AsyncWriteExt},
    net::unix::{gid_t, uid_t},
};

/// Backend selection for preferences (CoreFoundation vs File)
use crate::core::foundation;

/// Provides asynchronous operations for reading, writing, deleting, and managing
/// macOS plist preference files in user or global domains.
pub struct Preferences;

impl Preferences {
    /// Loads the plist from the given path.
    async fn load_plist(domain: &Domain) -> Result<LoadedPlist, PrefError> {
        use std::io::Cursor;

        let path = domain.get_path();
        let metadata = fs::metadata(&path).await;

        let orig_owner = match &metadata {
            Ok(m) => Some((m.uid(), m.gid())),
            Err(_) => None,
        };

        let (plist, is_binary) = if metadata.is_ok() {
            let mut file = File::open(&path).await.map_err(PrefError::Io)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.map_err(PrefError::Io)?;

            // try to parse as XML first, and if that fails, fallback to another format
            if let Ok(plist) = Value::from_reader_xml(Cursor::new(&buf[..])) {
                (plist, false)
            } else {
                let plist = Value::from_reader(Cursor::new(&buf[..]))
                    .map_err(|e| PrefError::Other(format!("Plist parse error: {e}")))?;
                (plist, true)
            }
        } else {
            // file does not exist, return a new empty dictionary value
            (Value::Dictionary(plist::Dictionary::new()), false)
        };

        Ok(LoadedPlist {
            plist: plist_to_prefvalue(&plist),
            orig_owner,
            is_binary,
        })
    }

    /// Saves the plist to the specified path and restores ownership using an atomic write.
    async fn save_plist(
        path: &PathBuf,
        plist: &PrefValue,
        orig_owner: Option<(u32, u32)>,
        is_binary: bool,
    ) -> Result<(), PrefError> {
        // acquire an exclusive lock on the file if possible
        let lock_file = fs::OpenOptions::new().read(true).open(path).await;
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
        let plist = prefvalue_to_plist(plist);

        if is_binary {
            plist
                .to_writer_binary(&mut buf)
                .map_err(|e| PrefError::Other(format!("Plist write error: {e}")))?;
        } else {
            plist
                .to_writer_xml(&mut buf)
                .map_err(|e| PrefError::Other(format!("Plist write error: {e}")))?;
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
            let _ = Self::restore_ownership(&tmp_path, uid, gid).await;
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
    async fn restore_ownership(path: &Path, uid: u32, gid: u32) -> Result<(), PrefError> {
        // obtain a reference to the path and retrieve its metadata (including permissions)
        let meta = fs::metadata(path).await.map_err(PrefError::Io)?;
        let orig_mode = meta.permissions().mode();

        // convert the file path to a null-terminated C string
        let c_path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| PrefError::Other("Invalid file path with interior null byte".into()))?;

        // safety: call libc::chown with a valid C string to change ownership
        let res = unsafe { libc::chown(c_path.as_ptr(), uid as uid_t, gid as gid_t) };

        if res == 0 {
            // restore the original file mode (permissions)
            set_permissions(path, Permissions::from_mode(orig_mode))
                .await
                .map_err(PrefError::Io)?;
            Ok(())
        } else {
            Err(PrefError::Io(std::io::Error::last_os_error()))
        }
    }

    /// List all available domains in ~/Library/Preferences.
    pub async fn list_domains() -> Result<Vec<Domain>, PrefError> {
        let list =
            foundation::list_domains().map_err(|_| PrefError::Other("CF list failed".into()))?;

        let domains: Vec<Domain> = list.iter().map(|f| Domain::User(f.to_string())).collect();
        Ok(domains)
    }

    /// Search all domains for keys or values containing the given word (case-insensitive).
    pub async fn find(word: &str) -> Result<BTreeMap<String, Vec<FindMatch>>, PrefError> {
        let word_lower = word.to_lowercase();
        let mut results: BTreeMap<String, Vec<FindMatch>> = BTreeMap::new();

        let domains: Vec<Domain> = Self::list_domains()
            .await?
            .into_iter()
            .chain([Domain::Global])
            .collect();

        for domain in domains {
            let loaded = match Self::load_plist(&domain).await {
                Ok(l) => l,
                Err(_) => continue,
            };

            let plist = loaded.plist;
            let mut matches = Vec::new();

            Self::find_in_value(&plist, &word_lower, String::new(), &mut matches);
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
                            key_path: new_key_path.clone(),
                            value: v.to_string(),
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
                let val_str = val.to_string();
                if contains_word(&val_str, word_lower) {
                    matches.push(FindMatch {
                        key_path,
                        value: val_str,
                    });
                }
            }
        }
    }

    /// Read a value from the given domain and key.
    ///
    /// If `key` is `None`, returns the entire domain as a `plist::Value`.
    /// If `key` is provided, returns the value at that key as a `PrefValue`.
    pub async fn read(domain: Domain, key: Option<&str>) -> Result<PrefValue, PrefError> {
        if matches!(domain, Domain::Path(_)) {
            return Self::file_read(domain, key).await;
        }

        let cf_name = &domain.get_cf_name();

        match foundation::read_pref(cf_name, key) {
            Some(pref_val) => {
                if key.is_some() {
                    Ok(pref_val)
                } else {
                    match pref_val {
                        PrefValue::Dictionary(_) => Ok(pref_val),
                        _ => Err(PrefError::Other(
                            "Expected dictionary for whole domain".into(),
                        )),
                    }
                }
            }
            None => Err(PrefError::KeyNotFound),
        }
    }

    async fn file_read(domain: Domain, key: Option<&str>) -> Result<PrefValue, PrefError> {
        let loaded = Self::load_plist(&domain).await?;
        let plist = loaded.plist;

        match key {
            Some(k) => {
                if let PrefValue::Dictionary(dict) = &plist {
                    match dict.get(k) {
                        Some(val) => Ok(val.clone()),
                        None => Err(PrefError::KeyNotFound),
                    }
                } else {
                    Err(PrefError::InvalidType)
                }
            }
            None => Ok(plist.clone()),
        }
    }

    /// Write a value to the given domain and key.
    ///
    /// If the domain file does not exist, it will be created.
    /// If the key already exists, its value will be overwritten.
    pub async fn write(domain: Domain, key: &str, value: PrefValue) -> Result<(), PrefError> {
        if matches!(domain, Domain::Path(_)) {
            return Self::file_write(domain, key, value).await;
        }
        let cf_name = &domain.get_cf_name();
        if foundation::write_pref(cf_name, key, &value) {
            Ok(())
        } else {
            Err(PrefError::Other("CFPreferences write failed".into()))
        }
    }

    async fn file_write(domain: Domain, key: &str, value: PrefValue) -> Result<(), PrefError> {
        let mut loaded = Self::load_plist(&domain).await?;

        if let PrefValue::Dictionary(ref mut dict) = loaded.plist {
            dict.insert(key.to_string(), value.clone());
        } else {
            return Err(PrefError::InvalidType);
        }
        Self::save_plist(
            &domain.get_path(),
            &loaded.plist,
            loaded.orig_owner,
            loaded.is_binary,
        )
        .await
    }

    /// Delete a key from the given domain.
    ///
    /// If `key` is `None`, deletes the entire domain file.
    /// If `key` is provided, removes the key from the domain plist.
    pub async fn delete(domain: Domain, key: Option<&str>) -> Result<(), PrefError> {
        if matches!(domain, Domain::Path(_)) {
            return Self::file_delete(domain, key).await;
        }
        let cf_name = &domain.get_cf_name();
        let ok = match key {
            Some(k) => foundation::delete_key(cf_name, k),
            None => foundation::delete_domain(cf_name),
        };
        if ok {
            Ok(())
        } else {
            Err(PrefError::Other("CFPreferences delete failed".into()))
        }
    }

    async fn file_delete(domain: Domain, key: Option<&str>) -> Result<(), PrefError> {
        let path = domain.get_path();
        match key {
            None => {
                if fs::metadata(&path).await.is_ok() {
                    fs::remove_file(&path).await.map_err(PrefError::Io)?;
                }
                Ok(())
            }
            Some(k) => {
                let mut loaded = Self::load_plist(&domain).await?;
                if let PrefValue::Dictionary(ref mut dict) = loaded.plist {
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
        let loaded = Self::load_plist(&domain).await?;
        let plist = loaded.plist;
        if let PrefValue::Dictionary(dict) = &plist {
            match dict.get(key) {
                Some(val) => Ok(val.get_type().to_string()),
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
        if matches!(domain, Domain::Path(_)) {
            return Self::file_rename(domain, old_key, new_key).await;
        }
        let cf_name = &domain.get_cf_name();
        // Read old value
        let val = foundation::read_pref(cf_name, Some(old_key)).ok_or(PrefError::KeyNotFound)?;
        // Write new key
        if !foundation::write_pref(cf_name, new_key, &val) {
            return Err(PrefError::Other("CFPreferences rename write failed".into()));
        }
        // Delete old key
        if !foundation::delete_key(cf_name, old_key) {
            return Err(PrefError::Other(
                "CFPreferences rename delete failed".into(),
            ));
        }
        Ok(())
    }

    async fn file_rename(domain: Domain, old_key: &str, new_key: &str) -> Result<(), PrefError> {
        let mut loaded = Self::load_plist(&domain).await?;

        if let PrefValue::Dictionary(ref mut dict) = loaded.plist {
            if let Some(val) = dict.remove(old_key) {
                dict.insert(new_key.to_string(), val);
                Self::save_plist(
                    &domain.get_path(),
                    &loaded.plist,
                    loaded.orig_owner,
                    loaded.is_binary,
                )
                .await
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
        if matches!(domain, Domain::Path(_)) {
            return Self::file_import(domain, import_path).await;
        }

        // Read plist file (async) then set keys via CF.
        let data = fs::read(import_path).await.map_err(PrefError::Io)?;
        // Try XML first
        let plist_val = if let Ok(v) = Value::from_reader_xml(std::io::Cursor::new(&data)) {
            v
        } else {
            Value::from_reader(std::io::Cursor::new(&data))
                .map_err(|e| PrefError::Other(format!("Plist parse error: {e}")))?
        };
        let dict = match plist_val {
            Value::Dictionary(d) => d,
            _ => {
                return Err(PrefError::Other(
                    "Import plist must be a dictionary at root".into(),
                ));
            }
        };

        let cf_name = &domain.get_cf_name();
        for (k, v) in dict {
            let pv = plist_to_prefvalue(&v);
            if !foundation::write_pref(cf_name, &k, &pv) {
                return Err(PrefError::Other(format!(
                    "Failed to import key `{k}` via CFPreferences"
                )));
            }
        }
        Ok(())
    }

    async fn file_import(domain: Domain, import_path: &str) -> Result<(), PrefError> {
        let dest_path = domain.get_path();
        let orig_owner = fs::metadata(&dest_path)
            .await
            .ok()
            .map(|m| (m.uid(), m.gid()));
        fs::copy(import_path, &dest_path)
            .await
            .map_err(PrefError::Io)?;
        if let Some((uid, gid)) = orig_owner {
            let _ = Self::restore_ownership(&dest_path, uid, gid).await;
        }
        Ok(())
    }

    /// Export a domain's plist file to the specified path.
    pub async fn export(domain: Domain, export_path: &str) -> Result<(), PrefError> {
        if matches!(domain, Domain::Path(_)) {
            return Self::file_export(domain, export_path).await;
        }
        let cf_name = &domain.get_cf_name();
        let pref = foundation::read_pref(cf_name, None)
            .unwrap_or(PrefValue::Dictionary(Default::default()));

        if !matches!(pref, PrefValue::Dictionary(_)) {
            return Err(PrefError::Other(
                "CF export produced non-dictionary root".into(),
            ));
        }
        // Save using existing atomic writer
        let path = PathBuf::from(export_path);

        // Determine original owner if exporting over an existing file
        let orig_owner = tokio::fs::metadata(&path)
            .await
            .ok()
            .map(|m| (m.uid(), m.gid()));

        Self::save_plist(&path, &pref, orig_owner, false).await
    }

    async fn file_export(domain: Domain, export_path: &str) -> Result<(), PrefError> {
        let src_path = domain.get_path();
        fs::copy(src_path, export_path)
            .await
            .map_err(PrefError::Io)?;
        Ok(())
    }

    /// Batch-write multiple key–value pairs for domains concurrently.
    ///
    /// # Concurrency & Grouping
    /// - The input is a vector of tuples `(Domain, String, PrefValue)`.
    /// - All write requests are grouped by domain.
    /// - For each domain, all key-value pairs are written in a single I/O operation.
    /// - All domains are processed concurrently using `futures::future::join_all`.
    ///
    /// # Behavior
    /// - Only the designated keys are updated in each plist; the entire domain is not replaced.
    /// - For CoreFoundation domains, each key is written individually.
    /// - For file-based domains, the plist is loaded, updated, and saved atomically.
    ///
    /// # Errors
    /// - If any write fails, the operation returns an error.
    pub async fn write_batch(batch: Vec<(Domain, String, PrefValue)>) -> Result<(), PrefError> {
        let mut groups: HashMap<Domain, Vec<(String, PrefValue)>> = HashMap::new();

        // Group write requests by domain.
        for (domain, key, value) in batch {
            groups.entry(domain).or_default().push((key, value));
        }

        let tasks = groups.into_iter().map(|(domain, writes)| async move {
            match &domain {
                Domain::User(_) | Domain::Global => {
                    let cf_name = &domain.get_cf_name();
                    for (key, value) in writes {
                        if !foundation::write_pref(cf_name, &key, &value) {
                            return Err(PrefError::Other(format!(
                                "CFPreferences write failed for key {}",
                                key
                            )));
                        }
                    }
                    Ok(())
                }
                Domain::Path(_) => {
                    let path = domain.get_path();
                    let mut loaded = Self::load_plist(&domain).await?;
                    if let PrefValue::Dictionary(ref mut dict) = loaded.plist {
                        for (key, value) in writes {
                            dict.insert(key, value);
                        }
                    } else {
                        return Err(PrefError::InvalidType);
                    }
                    Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary)
                        .await
                }
            }
        });

        for res in join_all(tasks).await {
            res?;
        }

        Ok(())
    }

    /// Batch-read multiple keys for domains concurrently.
    ///
    /// # Concurrency & Grouping
    /// - The input is a vector of tuples `(Domain, Option<String>)`.
    /// - Requests are grouped by domain.
    /// - For each domain, all requested keys are read in a single I/O operation.
    /// - All domains are processed concurrently using `futures::future::join_all`.
    ///
    /// # Behavior
    /// - If a request specifies `None` for the key, the entire domain plist is returned.
    /// - Otherwise, the value at the specified key is returned.
    /// - The result is a vector of tuples `(Domain, Option<String>, ReadResult)`.
    ///
    /// # Errors
    /// - If any read fails (e.g., key not found), the operation returns an error.
    #[allow(clippy::type_complexity)]
    pub async fn read_batch(
        batch: Vec<(Domain, Option<String>)>,
    ) -> Result<Vec<(Domain, Option<String>, PrefValue)>, PrefError> {
        let mut groups: HashMap<Domain, Vec<Option<String>>> = HashMap::new();

        // group requests by domain
        for (domain, key) in batch {
            groups.entry(domain).or_default().push(key);
        }

        // spawn concurrent futures to process each domain
        let futures = groups.into_iter().map(|(domain, keys)| async move {
            match &domain {
                Domain::User(_) | Domain::Global => {
                    let cf_name = &domain.get_cf_name();
                    let mut results = Vec::new();

                    for opt_key in keys {
                        match opt_key.as_deref() {
                            None => {
                                let pref_val = foundation::read_pref(cf_name, None)
                                    .unwrap_or(PrefValue::Dictionary(Default::default()));
                                results.push((domain.clone(), None, pref_val));
                            }
                            Some(k) => {
                                let pref_val = foundation::read_pref(cf_name, Some(k));
                                match pref_val {
                                    Some(val) => {
                                        results.push((domain.clone(), Some(k.to_string()), val))
                                    }
                                    None => return Err(PrefError::KeyNotFound),
                                }
                            }
                        }
                    }
                    Ok(results)
                }
                Domain::Path(_) => {
                    let loaded = Self::load_plist(&domain).await?;
                    let plist = loaded.plist;

                    let results: Result<Vec<_>, PrefError> = keys
                        .into_iter()
                        .map(|opt_key| match opt_key.as_deref() {
                            None => Ok((domain.clone(), None, plist.clone())),
                            Some(k) => {
                                let dict = match plist {
                                    PrefValue::Dictionary(ref d) => d,
                                    _ => return Err(PrefError::InvalidType),
                                };
                                let val = dict.get(k).ok_or(PrefError::KeyNotFound)?;
                                Ok((domain.clone(), Some(k.to_string()), val.clone()))
                            }
                        })
                        .collect();

                    results
                }
            }
        });

        // execute all domain reads concurrently
        let grouped_results: Result<Vec<Vec<(Domain, Option<String>, PrefValue)>>, _> =
            futures::future::join_all(futures)
                .await
                .into_iter()
                .collect();
        Ok(grouped_results?.into_iter().flatten().collect())
    }

    /// Batch-delete multiple keys for domains concurrently.
    ///
    /// # Concurrency & Grouping
    /// - The input is a vector of tuples `(Domain, Option<String>)`.
    /// - Requests are grouped by domain.
    /// - For each domain, all requested deletions are performed in a single I/O operation.
    /// - All domains are processed concurrently using `futures::future::join_all`.
    ///
    /// # Behavior
    /// - If any request for a domain has a key of `None`, the entire domain file is deleted.
    /// - Otherwise, only the specified keys are removed from the domain.
    ///
    /// # Errors
    /// - If any deletion fails (e.g., key not found), the operation returns an error.
    pub async fn delete_batch(batch: Vec<(Domain, Option<String>)>) -> Result<(), PrefError> {
        let mut groups: HashMap<Domain, Vec<Option<String>>> = HashMap::new();

        // group requests by domain
        for (domain, key) in batch {
            groups.entry(domain).or_default().push(key);
        }

        // spawn concurrent futures to process each domain deletion
        let futures = groups.into_iter().map(|(domain, keys)| async move {
            match &domain {
                Domain::User(_) | Domain::Global => {
                    let cf_name = &domain.get_cf_name();

                    if keys.iter().any(|k| k.is_none()) {
                        if !foundation::delete_domain(cf_name) {
                            return Err(PrefError::Other(
                                "CFPreferences delete domain failed".into(),
                            ));
                        }
                        Ok(())
                    } else {
                        for k in keys.into_iter().flatten() {
                            if !foundation::delete_key(cf_name, &k) {
                                return Err(PrefError::Other(format!(
                                    "CFPreferences delete failed for key {}",
                                    k
                                )));
                            }
                        }
                        Ok(())
                    }
                }
                Domain::Path(_) => {
                    if keys.iter().any(|k| k.is_none()) {
                        Self::delete(domain.clone(), None).await
                    } else {
                        let path = domain.get_path();
                        let mut loaded = Self::load_plist(&domain).await?;
                        if let PrefValue::Dictionary(ref mut dict) = loaded.plist {
                            for k in keys.into_iter().flatten() {
                                if dict.remove(&k).is_none() {
                                    return Err(PrefError::KeyNotFound);
                                }
                            }
                        } else {
                            return Err(PrefError::InvalidType);
                        }
                        Self::save_plist(&path, &loaded.plist, loaded.orig_owner, loaded.is_binary)
                            .await
                    }
                }
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
