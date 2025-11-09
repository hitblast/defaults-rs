use plist::Value;

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
    }
}
