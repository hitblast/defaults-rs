use crate::prefs::error::PrefError;
use crate::prefs::types::{Domain, PrefValue, ReadResult};
use plist::Value as PlistValue;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Preferences;

impl Preferences {
    /// Read a value from the given domain and key.
    /// If `key` is None, returns the entire domain as a plist::Value.
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
    pub async fn write(domain: Domain, key: &str, value: PrefValue) -> Result<(), PrefError> {
        let path = Self::domain_path(&domain);

        fn to_plist_value(val: &PrefValue) -> PlistValue {
            match val {
                PrefValue::String(s) => PlistValue::String(s.clone()),
                PrefValue::Integer(i) => PlistValue::Integer((*i).into()),
                PrefValue::Float(f) => PlistValue::Real(*f),
                PrefValue::Boolean(b) => PlistValue::Boolean(*b),
                PrefValue::Array(arr) => PlistValue::Array(arr.iter().map(to_plist_value).collect()),
                PrefValue::Dictionary(dict) => PlistValue::Dictionary(
                    dict.iter().map(|(k, v)| (k.clone(), to_plist_value(v))).collect()
                ),
            }
        }

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
            dict.insert(key.to_string(), to_plist_value(&value));
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

    /// Delete a key from the given domain.
    /// If `key` is None, deletes the entire domain file.
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
                        plist.to_writer_xml(&mut out_buf)
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
                plist.to_writer_xml(&mut out_buf)
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

    pub async fn import(domain: Domain, import_path: &str) -> Result<(), PrefError> {
        let dest_path = Self::domain_path(&domain);
        fs::copy(import_path, dest_path).await.map_err(PrefError::Io)?;
        Ok(())
    }

    pub async fn export(domain: Domain, export_path: &str) -> Result<(), PrefError> {
        let src_path = Self::domain_path(&domain);
        fs::copy(src_path, export_path).await.map_err(PrefError::Io)?;
        Ok(())
    }

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

    /// Pretty-print a PlistValue in Apple-style format (for CLI).
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

    fn quote_key(key: &str) -> String {
        if key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            key.to_string()
        } else {
            format!("\"{}\"", key.replace('"', "\\\""))
        }
    }

    fn quote_string(s: &str) -> String {
        if s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\\\""))
        }
    }
}