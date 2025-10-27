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
    base::{CFGetTypeID, CFTypeRef, TCFType},
    boolean::{CFBooleanGetTypeID, kCFBooleanFalse, kCFBooleanTrue},
    number::CFNumber,
    string::CFString,
};

use core_foundation_sys::{
    array::{
        CFArrayCreate, CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex,
        kCFTypeArrayCallBacks,
    },
    base::kCFAllocatorDefault,
    dictionary::{
        CFDictionaryCreate, CFDictionaryGetCount, CFDictionaryGetKeysAndValues,
        CFDictionaryGetTypeID, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
    },
    number::{
        CFNumberCreate, CFNumberGetType, CFNumberGetTypeID, CFNumberGetValue, kCFNumberDoubleType,
        kCFNumberSInt64Type,
    },
    preferences::{
        CFPreferencesAppSynchronize, CFPreferencesCopyAppValue, CFPreferencesCopyApplicationList,
        CFPreferencesCopyKeyList, CFPreferencesSetAppValue, kCFPreferencesAnyHost,
        kCFPreferencesCurrentUser,
    },
    string::CFStringGetTypeID,
};

use crate::prefs::types::PrefValue;

/// Public entry to confirm CF backend viability.
pub fn cf_available() -> bool {
    list_domains().is_ok()
}

/// List all preference application IDs (domains) for CurrentUser / AnyHost.
pub fn list_domains() -> Result<Vec<String>, ()> {
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
pub fn read_pref(domain: &str, key: Option<&str>) -> Option<PrefValue> {
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
                Some(cf_any_to_pref(raw as _))
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
                        map.insert(key_cf.to_string(), cf_any_to_pref(raw as _));
                    }
                }
                Some(PrefValue::Dictionary(map))
            }
        }
    }
}

/// Write (set) a single key in a domain. Returns success (synchronize result).
pub fn write_pref(domain: &str, key: &str, value: &PrefValue) -> bool {
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
pub fn delete_key(domain: &str, key: &str) -> bool {
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
pub fn delete_domain(domain: &str) -> bool {
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

// Conversion helpers
//
// These must only be used by the module itself;
// exporting of these functions is prohibited.

unsafe fn cfboolean_to_bool(r: CFTypeRef) -> Option<bool> {
    // Capture the canonical true/false CFBoolean refs once, then compare.
    let true_ref = unsafe { kCFBooleanTrue } as CFTypeRef;
    let false_ref = unsafe { kCFBooleanFalse } as CFTypeRef;
    if r == true_ref {
        Some(true)
    } else if r == false_ref {
        Some(false)
    } else {
        None
    }
}

unsafe fn cfnumber_to_pref(r: CFTypeRef) -> Option<PrefValue> {
    use core_foundation_sys::number::CFNumberType;
    let num = unsafe { CFNumber::wrap_under_get_rule(r as _) };
    let ntype: CFNumberType = unsafe { CFNumberGetType(num.as_concrete_TypeRef()) };
    let mut i64_val: i64 = 0;
    let got_int = unsafe {
        CFNumberGetValue(
            num.as_concrete_TypeRef(),
            kCFNumberSInt64Type as CFNumberType,
            &mut i64_val as *mut i64 as *mut _,
        ) as i32
            != 0
    };
    if got_int && ntype == kCFNumberSInt64Type {
        return Some(PrefValue::Integer(i64_val));
    }
    let mut f64_val: f64 = 0.0;
    let got_float = unsafe {
        CFNumberGetValue(
            num.as_concrete_TypeRef(),
            kCFNumberDoubleType as CFNumberType,
            &mut f64_val as *mut f64 as *mut _,
        ) as i32
            != 0
    };
    if got_float {
        return Some(PrefValue::Float(f64_val));
    }
    None
}

unsafe fn cfarray_to_pref(r: CFTypeRef) -> Option<PrefValue> {
    let len = unsafe { CFArrayGetCount(r as _) };
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        let item = unsafe { CFArrayGetValueAtIndex(r as _, i) };
        if !item.is_null() {
            out.push(unsafe { cf_any_to_pref(item as _) });
        }
    }
    Some(PrefValue::Array(out))
}

unsafe fn cfdict_to_pref(r: CFTypeRef) -> Option<PrefValue> {
    let count = unsafe { CFDictionaryGetCount(r as _) };
    if count == 0 {
        return Some(PrefValue::Dictionary(HashMap::new()));
    }
    let mut keys: Vec<CFTypeRef> = Vec::with_capacity(count as usize);
    let mut vals: Vec<CFTypeRef> = Vec::with_capacity(count as usize);
    unsafe {
        keys.set_len(count as usize);
        vals.set_len(count as usize);
        CFDictionaryGetKeysAndValues(
            r as _,
            keys.as_mut_ptr() as *mut _,
            vals.as_mut_ptr() as *mut _,
        );
    }
    let mut map = HashMap::new();
    for i in 0..count as usize {
        let kref = keys[i];
        if !kref.is_null() && unsafe { CFGetTypeID(kref as _) } == unsafe { CFStringGetTypeID() } {
            let key = unsafe { CFString::wrap_under_get_rule(kref as _).to_string() };
            let vref = vals[i];
            if !vref.is_null() {
                map.insert(key, unsafe { cf_any_to_pref(vref as _) });
            }
        }
    }
    Some(PrefValue::Dictionary(map))
}

unsafe fn cf_any_to_pref(r: CFTypeRef) -> PrefValue {
    let tid = unsafe { CFGetTypeID(r) };
    let string_tid = unsafe { CFStringGetTypeID() };
    let bool_tid = unsafe { CFBooleanGetTypeID() };
    let num_tid = unsafe { CFNumberGetTypeID() };
    let arr_tid = unsafe { CFArrayGetTypeID() };
    let dict_tid = unsafe { CFDictionaryGetTypeID() };

    if tid == string_tid {
        PrefValue::String(unsafe { CFString::wrap_under_get_rule(r as _).to_string() })
    } else if tid == bool_tid {
        unsafe {
            cfboolean_to_bool(r)
                .map(PrefValue::Boolean)
                .unwrap_or_else(|| PrefValue::String("<invalid bool>".into()))
        }
    } else if tid == num_tid {
        unsafe {
            cfnumber_to_pref(r).unwrap_or_else(|| PrefValue::String("<invalid number>".into()))
        }
    } else if tid == arr_tid {
        unsafe {
            cfarray_to_pref(r).unwrap_or_else(|| PrefValue::String("<array conv error>".into()))
        }
    } else if tid == dict_tid {
        unsafe {
            cfdict_to_pref(r).unwrap_or_else(|| PrefValue::String("<dict conv error>".into()))
        }
    } else {
        PrefValue::String("<unsupported CF type>".into())
    }
}

fn pref_to_cf(value: &PrefValue) -> CFTypeRef {
    match value {
        PrefValue::String(s) => CFString::new(s).as_concrete_TypeRef() as _,
        PrefValue::Integer(i) => unsafe {
            CFNumberCreate(
                kCFAllocatorDefault,
                kCFNumberSInt64Type,
                i as *const i64 as *const _,
            ) as CFTypeRef
        },
        PrefValue::Float(f) => unsafe {
            CFNumberCreate(
                kCFAllocatorDefault,
                kCFNumberDoubleType,
                f as *const f64 as *const _,
            ) as CFTypeRef
        },
        PrefValue::Boolean(b) => unsafe {
            if *b {
                kCFBooleanTrue as CFTypeRef
            } else {
                kCFBooleanFalse as CFTypeRef
            }
        },
        PrefValue::Array(items) => unsafe {
            let mut cf_items: Vec<CFTypeRef> = items.iter().map(pref_to_cf).collect();
            CFArrayCreate(
                kCFAllocatorDefault,
                cf_items.as_mut_ptr() as *const _,
                cf_items.len() as isize,
                &kCFTypeArrayCallBacks,
            ) as CFTypeRef
        },
        PrefValue::Dictionary(map) => unsafe {
            let key_strings: Vec<CFString> = map.keys().map(|k| CFString::new(k)).collect();
            let mut keys: Vec<CFTypeRef> = key_strings
                .iter()
                .map(|k| k.as_concrete_TypeRef() as CFTypeRef)
                .collect();
            let mut values: Vec<CFTypeRef> = map.values().map(pref_to_cf).collect();
            CFDictionaryCreate(
                kCFAllocatorDefault,
                keys.as_mut_ptr() as *const _,
                values.as_mut_ptr() as *const _,
                keys.len() as isize,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            ) as CFTypeRef
        },
    }
}
