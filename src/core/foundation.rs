// SPDX-License-Identifier: MIT

//! CoreFoundation-based preferences backend (cleaned imports & unsafe usage).
//!
//! Provides minimal CFPreferences integration:
//! - Domain listing
//! - Single key read / whole domain read
//! - Write key
//! - Delete key / whole domain
//!
//! Supports scalar, array and dictionary types (best-effort). Binary/data
//! types are converted to placeholder strings for now.

use std::collections::HashMap;

use core_foundation::{
    base::{CFGetTypeID, TCFType},
    string::CFString,
};

use core_foundation_sys::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex},
    preferences::{
        CFPreferencesAppSynchronize, CFPreferencesCopyAppValue, CFPreferencesCopyApplicationList,
        CFPreferencesCopyKeyList, CFPreferencesSetAppValue, kCFPreferencesAnyHost,
        kCFPreferencesCurrentUser,
    },
    string::CFStringGetTypeID,
};

use crate::core::{
    convert::{cf_to_pref, pref_to_cf},
    types::PrefValue,
};

/// List all preference application IDs (domains) for CurrentUser / AnyHost.
pub(crate) fn list_domains() -> Result<Vec<String>, ()> {
    unsafe {
        let arr_ref =
            CFPreferencesCopyApplicationList(kCFPreferencesCurrentUser, kCFPreferencesAnyHost);
        if arr_ref.is_null() {
            return Ok(Vec::new());
        }
        let len = CFArrayGetCount(arr_ref);
        let mut out = Vec::with_capacity(len as usize);
        for i in 0..len {
            let val = CFArrayGetValueAtIndex(arr_ref, i);
            if !val.is_null() && CFGetTypeID(val as _) == CFStringGetTypeID() {
                let s = CFString::wrap_under_get_rule(val as _);
                out.push(s.to_string());
            }
        }
        out.sort();
        Ok(out)
    }
}

/// Read either a single key or the whole domain as PrefValue.
pub(crate) fn read_pref(domain: &str, key: Option<&str>) -> Option<PrefValue> {
    unsafe {
        let domain_cf = CFString::new(domain);
        match key {
            Some(k) => {
                let key_cf = CFString::new(k);
                let raw = CFPreferencesCopyAppValue(
                    key_cf.as_concrete_TypeRef(),
                    domain_cf.as_concrete_TypeRef(),
                );
                if raw.is_null() {
                    return None;
                }
                Some(cf_to_pref(raw as _))
            }
            None => {
                let keys_ref = CFPreferencesCopyKeyList(
                    domain_cf.as_concrete_TypeRef(),
                    kCFPreferencesCurrentUser,
                    kCFPreferencesAnyHost,
                );
                if keys_ref.is_null() {
                    return Some(PrefValue::Dictionary(HashMap::new()));
                }
                let len = CFArrayGetCount(keys_ref);
                let mut map = HashMap::new();
                for i in 0..len {
                    let key_ref = CFArrayGetValueAtIndex(keys_ref, i);
                    if key_ref.is_null() || CFGetTypeID(key_ref as _) != CFStringGetTypeID() {
                        continue;
                    }
                    let key_cf = CFString::wrap_under_get_rule(key_ref as _);
                    let raw = CFPreferencesCopyAppValue(
                        key_cf.as_concrete_TypeRef(),
                        domain_cf.as_concrete_TypeRef(),
                    );
                    if !raw.is_null() {
                        map.insert(key_cf.to_string(), cf_to_pref(raw as _));
                    }
                }
                Some(PrefValue::Dictionary(map))
            }
        }
    }
}

/// Write (set) a single key in a domain. Returns success (synchronize result).
pub(crate) fn write_pref(domain: &str, key: &str, value: &PrefValue) -> bool {
    unsafe {
        let domain_cf = CFString::new(domain);
        let key_cf = CFString::new(key);
        let value_ref = pref_to_cf(value);
        CFPreferencesSetAppValue(
            key_cf.as_concrete_TypeRef(),
            value_ref,
            domain_cf.as_concrete_TypeRef(),
        );
        CFPreferencesAppSynchronize(domain_cf.as_concrete_TypeRef()) != 0
    }
}

/// Delete a single key. Returns success (including if key absent).
pub(crate) fn delete_key(domain: &str, key: &str) -> bool {
    unsafe {
        let domain_cf = CFString::new(domain);
        let key_cf = CFString::new(key);
        CFPreferencesSetAppValue(
            key_cf.as_concrete_TypeRef(),
            std::ptr::null(),
            domain_cf.as_concrete_TypeRef(),
        );
        CFPreferencesAppSynchronize(domain_cf.as_concrete_TypeRef()) != 0
    }
}

/// Delete all keys in a domain.
pub(crate) fn delete_domain(domain: &str) -> bool {
    if let Some(PrefValue::Dictionary(keys)) = read_pref(domain, None) {
        let mut ok = true;
        for k in keys.keys() {
            if !delete_key(domain, k) {
                ok = false;
            }
        }
        ok
    } else {
        true
    }
}
