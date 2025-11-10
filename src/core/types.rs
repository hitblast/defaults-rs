// SPDX-License-Identifier: MIT

//! This module defines the types for representing preferences.
//!
//! The batch operations in the API (batch-read and batch-delete) work on the [`Domain`] and [`PrefValue`] types.

use once_cell::sync::Lazy;
use std::{collections::HashMap, path::PathBuf};

static HOME: Lazy<String> = Lazy::new(|| dirs::home_dir().unwrap().display().to_string());

/// Preferences domain (user or global).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Domain {
    /// A user domain, e.g., "com.apple.finder"
    User(String),
    /// The global preferences domain (".GlobalPreferences")
    Global,
    /// A direct path to a plist file
    Path(std::path::PathBuf),
}

impl Domain {
    /// Returns the filesystem path for a given domain.
    pub fn get_path(&self) -> PathBuf {
        match &self {
            Domain::Global => PathBuf::from(format!(
                "{}/Library/Preferences/.GlobalPreferences.plist",
                *HOME
            )),
            Domain::User(name) => {
                PathBuf::from(format!("{}/Library/Preferences/{}.plist", *HOME, name))
            }
            Domain::Path(path) => path.clone(),
        }
    }

    /// Returns the CoreFoundation name for a given domain.
    pub fn get_cf_name(&self) -> String {
        match &self {
            Domain::Global => String::from(".GlobalPreferences"),
            Domain::User(name) => name.clone(),
            Domain::Path(_) => unreachable!("no CF name for path-based domains"),
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::User(s) => write!(f, "{}", s),
            Domain::Global => write!(f, "NSGlobalDomain"),
            Domain::Path(p) => write!(f, "{}", p.display()),
        }
    }
}

/// Value stored in preferences.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<PrefValue>),
    Dictionary(HashMap<String, PrefValue>),
    Data(Vec<u8>), // CFData
    Date(f64),     // CFDate (CFAbsoluteTime)
    Url(String),   // CFURL
    Uuid(String),  // CFUUID
    Uid(u64),
}

impl Default for PrefValue {
    fn default() -> Self {
        PrefValue::String(String::default())
    }
}

impl std::fmt::Display for PrefValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrefValue::Boolean(b) => write!(f, "{}", b),
            PrefValue::Integer(i) => write!(f, "{}", i),
            PrefValue::Float(fl) => write!(f, "{}", fl),
            PrefValue::String(s) => write!(f, "{}", s),
            PrefValue::Array(arr) => {
                write!(
                    f,
                    "[{}]",
                    arr.iter()
                        .map(|v| format!("{}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            PrefValue::Dictionary(dict) => {
                write!(
                    f,
                    "{{{}}}",
                    dict.iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            PrefValue::Data(data) => {
                write!(f, "<Data: {} bytes>", data.len())
            }
            PrefValue::Date(dt) => {
                write!(f, "<Date: {}>", dt)
            }
            PrefValue::Url(url) => {
                write!(f, "<Url: {}>", url)
            }
            PrefValue::Uuid(uuid) => {
                write!(f, "<Uuid: {}>", uuid)
            }
            PrefValue::Uid(uid) => {
                write!(f, "<Uid: {}>", uid)
            }
        }
    }
}

impl PrefValue {
    /// Returns the name of the type for the PrefValue instance.
    pub fn get_type(&self) -> &'static str {
        match self {
            PrefValue::String(_) => "string",
            PrefValue::Integer(_) => "integer",
            PrefValue::Float(_) => "float",
            PrefValue::Boolean(_) => "boolean",
            PrefValue::Array(_) => "array",
            PrefValue::Dictionary(_) => "dictionary",
            PrefValue::Data(_) => "data",
            PrefValue::Date(_) => "date",
            PrefValue::Url(_) => "url",
            PrefValue::Uuid(_) => "uuid",
            PrefValue::Uid(_) => "uid",
        }
    }
}

/// Result of a find operation.
#[derive(Debug)]
pub struct FindMatch {
    pub key_path: String,
    pub value: String,
}

/// Struct representing a loaded plist, including its original owner and whether it was read as binary.
#[derive(Debug)]
pub struct LoadedPlist {
    pub plist: PrefValue,
    pub orig_owner: Option<(u32, u32)>,
    pub is_binary: bool,
}
