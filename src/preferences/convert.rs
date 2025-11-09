use plist::Value;
use std::time::{Duration, UNIX_EPOCH};

use crate::PrefValue;

pub(crate) fn plist_to_prefvalue(val: &Value) -> PrefValue {
    match val {
        Value::String(s) => PrefValue::String(s.clone()),
        Value::Integer(i) => PrefValue::Integer(i.as_signed().unwrap_or(0)),
        Value::Real(f) => PrefValue::Float(*f),
        Value::Boolean(b) => PrefValue::Boolean(*b),
        Value::Array(arr) => PrefValue::Array(arr.iter().map(plist_to_prefvalue).collect()),
        Value::Dictionary(dict) => PrefValue::Dictionary(
            dict.iter()
                .map(|(k, v)| (k.clone(), plist_to_prefvalue(v)))
                .collect(),
        ),
        _ => PrefValue::String(format!("{val:?}")),
    }
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
        PrefValue::Data(data) => Value::Data(data.clone()),
        PrefValue::Date(dt) => {
            // Apple epoch is Jan 1, 2001, which is 978307200 seconds after UNIX_EPOCH
            const APPLE_EPOCH_UNIX: u64 = 978307200;
            let secs = APPLE_EPOCH_UNIX as f64 + *dt;
            let system_time = UNIX_EPOCH
                + Duration::from_secs(secs as u64)
                + Duration::from_nanos((secs.fract() * 1e9) as u64);
            Value::Date(plist::Date::from(system_time))
        }
        PrefValue::Url(url) => Value::String(url.clone()),
        PrefValue::Uuid(uuid) => Value::String(uuid.clone()),
    }
}
