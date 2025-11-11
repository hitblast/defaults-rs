// SPDX-License-Identifier: MIT

//! CoreFoundation-based preferences backend (cleaned imports & unsafe usage).
//!
//! Provides minimal CFPreferences integration:
//! - Domain listing
//! - Single key read / whole domain read
//! - Write key
//! - Delete key / whole domain

use anyhow::Result;
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
pub(crate) fn list_domains() -> Result<Vec<String>> {
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

use anyhow::bail;

/// Read a single key as PrefValue.
pub(crate) fn read_pref(domain: &str, key: &str) -> Result<PrefValue> {
    unsafe {
        let domain_cf = CFString::new(domain);
        let key_cf = CFString::new(key);
        let raw = CFPreferencesCopyAppValue(
            key_cf.as_concrete_TypeRef(),
            domain_cf.as_concrete_TypeRef(),
        );
        if raw.is_null() {
            bail!("Key not found for domain {domain}: {key}");
        }
        Ok(cf_to_pref(raw as _))
    }
}

/// Read the whole domain as PrefValue::Dictionary.
pub(crate) fn read_pref_domain(domain: &str) -> Result<PrefValue> {
    unsafe {
        let domain_cf = CFString::new(domain);
        let keys_ref = CFPreferencesCopyKeyList(
            domain_cf.as_concrete_TypeRef(),
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        );
        if keys_ref.is_null() {
            return Ok(PrefValue::Dictionary(HashMap::new()));
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
        Ok(PrefValue::Dictionary(map))
    }
}

/// Write (set) a single key in a domain. Returns success (synchronize result).
pub(crate) fn write_pref(domain: &str, key: &str, value: &PrefValue) -> Result<()> {
    unsafe {
        let domain_cf = CFString::new(domain);
        let key_cf = CFString::new(key);
        let value_ref = pref_to_cf(value);
        CFPreferencesSetAppValue(
            key_cf.as_concrete_TypeRef(),
            value_ref,
            domain_cf.as_concrete_TypeRef(),
        );
        if CFPreferencesAppSynchronize(domain_cf.as_concrete_TypeRef()) != 0 {
            Ok(())
        } else {
            bail!("Failed to write key: {}", key)
        }
    }
}

/// Delete a single key. Returns success (including if key absent).
pub(crate) fn delete_key(domain: &str, key: &str) -> Result<()> {
    unsafe {
        let domain_cf = CFString::new(domain);
        let key_cf = CFString::new(key);
        CFPreferencesSetAppValue(
            key_cf.as_concrete_TypeRef(),
            std::ptr::null(),
            domain_cf.as_concrete_TypeRef(),
        );
        if CFPreferencesAppSynchronize(domain_cf.as_concrete_TypeRef()) != 0 {
            Ok(())
        } else {
            bail!("Failed to delete key: {}", key)
        }
    }
}

/// Delete all keys in a domain.
pub(crate) fn delete_domain(domain: &str) -> Result<()> {
    let loaded = read_pref_domain(domain)?;

    match loaded {
        PrefValue::Dictionary(keys) => {
            for k in keys.keys() {
                delete_key(domain, k)?;
            }

            Ok(())
        }
        _ => bail!("Cannot delete a domain which is not a dictionary."),
    }
}
