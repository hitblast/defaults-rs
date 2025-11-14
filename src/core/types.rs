// SPDX-License-Identifier: MIT

//! This module defines the types for representing preferences.
//!
//! The batch operations in the API (batch-read and batch-delete) work on the [`Domain`] and [`PrefValue`] types.

use std::collections::HashMap;

/// Value stored in preferences.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<PrefValue>),
    Dictionary(HashMap<String, PrefValue>),
    Data(Vec<u8>),
    Date(f64),
    Url(String),
    Uuid(String),
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
                write!(f, "<Data: length {} bytes>", data.len())
            }
            PrefValue::Date(apple_ts) => {
                write!(f, "<Date: {}>", {
                    use chrono::{TimeZone, Utc};

                    let base = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).unwrap();
                    let secs = *apple_ts as i64;
                    let nanos = ((apple_ts % 1.0) * 1e9) as u32;

                    base + chrono::Duration::seconds(secs)
                        + chrono::Duration::nanoseconds(nanos as i64)
                })
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
