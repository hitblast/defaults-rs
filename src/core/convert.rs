// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use core_foundation::{
    array::{
        CFArrayCreate, CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex,
        kCFTypeArrayCallBacks,
    },
    base::{CFGetTypeID, CFRelease, CFRetain, CFTypeRef, TCFType, kCFAllocatorDefault},
    data::{CFDataCreate, CFDataGetBytePtr, CFDataGetLength, CFDataGetTypeID},
    date::{CFDateCreate, CFDateGetAbsoluteTime, CFDateGetTypeID},
    dictionary::{
        CFDictionaryCreate, CFDictionaryGetCount, CFDictionaryGetKeysAndValues,
        CFDictionaryGetTypeID, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
    },
    number::{
        CFBooleanGetTypeID, CFNumber, CFNumberCreate, CFNumberGetType, CFNumberGetTypeID,
        CFNumberGetValue, kCFBooleanFalse, kCFBooleanTrue, kCFNumberDoubleType,
        kCFNumberSInt64Type,
    },
    string::{CFString, CFStringGetTypeID},
    url::{CFURLCreateWithString, CFURLGetString, CFURLGetTypeID},
    uuid::{CFUUIDCreateFromString, CFUUIDCreateString, CFUUIDGetTypeID},
};

use crate::PrefValue;

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
            out.push(unsafe { cf_to_pref(item as _) });
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
                map.insert(key, unsafe { cf_to_pref(vref as _) });
            }
        }
    }
    Some(PrefValue::Dictionary(map))
}

pub(crate) unsafe fn cf_to_pref(r: CFTypeRef) -> PrefValue {
    let tid = unsafe { CFGetTypeID(r) };
    let string_tid = unsafe { CFStringGetTypeID() };
    let bool_tid = unsafe { CFBooleanGetTypeID() };
    let num_tid = unsafe { CFNumberGetTypeID() };
    let arr_tid = unsafe { CFArrayGetTypeID() };
    let dict_tid = unsafe { CFDictionaryGetTypeID() };
    let data_tid = unsafe { CFDataGetTypeID() };
    let date_tid = unsafe { CFDateGetTypeID() };
    let url_tid = unsafe { CFURLGetTypeID() };
    let uuid_tid = unsafe { CFUUIDGetTypeID() };

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
    } else if tid == data_tid {
        let len = unsafe { CFDataGetLength(r as _) };
        let ptr = unsafe { CFDataGetBytePtr(r as _) };
        let data = unsafe { std::slice::from_raw_parts(ptr, len as usize).to_vec() };
        PrefValue::Data(data)
    } else if tid == date_tid {
        let abs_time = unsafe { CFDateGetAbsoluteTime(r as _) };
        PrefValue::Date(abs_time)
    } else if tid == url_tid {
        let cfstr = unsafe { CFURLGetString(r as _) };
        let url = unsafe { CFString::wrap_under_get_rule(cfstr as _).to_string() };
        PrefValue::Url(url)
    } else if tid == uuid_tid {
        let cfstr = unsafe { CFUUIDCreateString(kCFAllocatorDefault, r as _) };
        let uuid = unsafe { CFString::wrap_under_get_rule(cfstr as _).to_string() };
        PrefValue::Uuid(uuid)
    } else {
        PrefValue::String("<unsupported CF type>".into())
    }
}

pub(crate) fn pref_to_cf(value: &PrefValue) -> CFTypeRef {
    match value {
        PrefValue::String(s) => {
            let cs = CFString::new(s);
            let ptr = cs.as_concrete_TypeRef();
            unsafe { CFRetain(ptr as *const _ as *mut _) };
            ptr as CFTypeRef
        }

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
            (if *b { kCFBooleanTrue } else { kCFBooleanFalse }) as CFTypeRef
        },

        PrefValue::Array(items) => unsafe {
            let mut cf_items: Vec<CFTypeRef> = items.iter().map(pref_to_cf).collect();
            let arr = CFArrayCreate(
                kCFAllocatorDefault,
                cf_items.as_mut_ptr() as *const _,
                cf_items.len() as isize,
                &kCFTypeArrayCallBacks,
            ) as CFTypeRef;

            for &it in &cf_items {
                CFRelease(it as *const _ as *mut _);
            }
            arr
        },

        PrefValue::Dictionary(map) => unsafe {
            let key_cfs: Vec<CFString> = map.keys().map(|k| CFString::new(k)).collect();
            let mut keys: Vec<CFTypeRef> = key_cfs
                .iter()
                .map(|k| {
                    let p = k.as_concrete_TypeRef() as CFTypeRef;
                    CFRetain(p as *const _ as *mut _);
                    p
                })
                .collect();

            let mut values: Vec<CFTypeRef> = map.values().map(pref_to_cf).collect();

            let dict = CFDictionaryCreate(
                kCFAllocatorDefault,
                keys.as_mut_ptr() as *const _,
                values.as_mut_ptr() as *const _,
                keys.len() as isize,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            ) as CFTypeRef;

            for &k in &keys {
                CFRelease(k as *const _ as *mut _);
            }
            for &v in &values {
                CFRelease(v as *const _ as *mut _);
            }

            dict
        },

        PrefValue::Data(data) => unsafe {
            CFDataCreate(kCFAllocatorDefault, data.as_ptr(), data.len() as isize) as CFTypeRef
        },

        PrefValue::Date(dt) => unsafe { CFDateCreate(kCFAllocatorDefault, *dt) as CFTypeRef },

        PrefValue::Url(url) => unsafe {
            let cf_url_str = CFString::new(url);
            CFURLCreateWithString(
                kCFAllocatorDefault,
                cf_url_str.as_concrete_TypeRef(),
                std::ptr::null(),
            ) as CFTypeRef
        },

        PrefValue::Uuid(uuid) => unsafe {
            let cf_uuid_str = CFString::new(uuid);
            CFUUIDCreateFromString(kCFAllocatorDefault, cf_uuid_str.as_concrete_TypeRef())
                as CFTypeRef
        },
    }
}
