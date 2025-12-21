use anyhow::{Context, Result, bail};
use plist::{Uid, Value};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::PrefValue;

// Apple epoch is Jan 1, 2001, which is 978307200 seconds after UNIX_EPOCH
static APPLE_EPOCH_UNIX: u64 = 978307200;

pub(crate) fn plist_to_prefvalue(val: &Value) -> Result<PrefValue> {
    let val = match val {
        Value::String(s) => PrefValue::String(s.clone()),
        Value::Integer(i) => PrefValue::Integer(i.as_signed().unwrap_or(0)),
        Value::Real(f) => PrefValue::Float(*f),
        Value::Boolean(b) => PrefValue::Boolean(*b),
        Value::Array(arr) => {
            let mut result = Vec::new();
            for f in arr.iter() {
                result.push(plist_to_prefvalue(f)?);
            }
            PrefValue::Array(result)
        }
        Value::Dictionary(dict) => {
            let mut result = HashMap::new();

            for (k, v) in dict.iter() {
                result.insert(k.clone(), plist_to_prefvalue(v)?);
            }
            PrefValue::Dictionary(result)
        }
        Value::Data(data) => PrefValue::Data(data.clone().into_boxed_slice()),
        Value::Date(date) => {
            let system_time: SystemTime = date.clone().into();
            let duration_since_unix = system_time
                .duration_since(UNIX_EPOCH)
                .context("Failed to calculate duration since UNIX_EPOCH when converting.")?
                .as_secs_f64();
            let seconds_since_apple_epoch = duration_since_unix - APPLE_EPOCH_UNIX as f64;
            PrefValue::Date(seconds_since_apple_epoch)
        }
        Value::Uid(uid) => PrefValue::Uid(uid.get()),
        _ => bail!("Cannot reach this conversion for Value type."),
    };

    Ok(val)
}

pub(crate) fn prefvalue_to_plist(val: &PrefValue) -> Value {
    match val {
        PrefValue::String(s) => Value::String(s.clone()),
        PrefValue::Integer(i) => Value::Integer((*i).into()),
        PrefValue::Float(f) => Value::Real(*f),
        PrefValue::Boolean(b) => Value::Boolean(*b),
        PrefValue::Array(arr) => Value::Array(arr.iter().map(prefvalue_to_plist).collect()),
        PrefValue::Dictionary(dict) => Value::Dictionary(
            dict.iter()
                .map(|(k, v)| (k.clone(), prefvalue_to_plist(v)))
                .collect(),
        ),
        PrefValue::Data(data) => Value::Data(data.clone().to_vec()),
        PrefValue::Date(dt) => {
            let secs = APPLE_EPOCH_UNIX as f64 + *dt;
            let system_time = UNIX_EPOCH
                + Duration::from_secs(secs as u64)
                + Duration::from_nanos((secs.fract() * 1e9) as u64);
            Value::Date(plist::Date::from(system_time))
        }
        PrefValue::Url(url) => Value::String(url.clone()),
        PrefValue::Uuid(uuid) => Value::String(uuid.clone()),
        PrefValue::Uid(uid) => Value::Uid(Uid::new(*uid)),
    }
}
