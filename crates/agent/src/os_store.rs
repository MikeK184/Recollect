//! macOS Keychain items that trust our own binaries.
//!
//! The generic credential store prompts on every access because its items
//! carry no trusted-application list. These items name the companion, bridge,
//! and runner binaries explicitly, so a single pairing approval covers later
//! setup, run, and bridge loads by those binaries. Same service/account shape
//! as the generic store, so reads find previously paired items.

use anyhow::{Result, anyhow};
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, OSStatus, TCFType},
    data::{CFData, CFDataRef},
    dictionary::{CFDictionary, CFDictionaryRef},
    string::{CFString, CFStringRef},
};
use core_foundation_sys::base::CFRelease;
use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    ptr,
};

type SecAccessRef = *const c_void;
type SecTrustedApplicationRef = *const c_void;

const SUCCESS: OSStatus = 0;
const ITEM_NOT_FOUND: OSStatus = -25300;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecClass: CFTypeRef;
    static kSecClassGenericPassword: CFTypeRef;
    static kSecAttrService: CFTypeRef;
    static kSecAttrAccount: CFTypeRef;
    static kSecValueData: CFTypeRef;
    static kSecReturnData: CFTypeRef;
    static kSecMatchLimit: CFTypeRef;
    static kSecMatchLimitOne: CFTypeRef;
    static kSecAttrAccess: CFTypeRef;
    fn SecAccessCreate(
        descriptor: CFStringRef,
        trustedList: CFArrayRef,
        owner: *const c_void,
        accessRef: *mut SecAccessRef,
    ) -> OSStatus;
    fn SecTrustedApplicationCreateFromPath(
        path: CFStringRef,
        trustedApp: *mut SecTrustedApplicationRef,
    ) -> OSStatus;
    fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: CFTypeRef;
}

struct Release(*const c_void);
impl Drop for Release {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0 as CFTypeRef) };
        }
    }
}

fn unavailable() -> anyhow::Error {
    anyhow!(
        "The OS credential store is unavailable or locked. Unlock it and retry; no file fallback is used."
    )
}

/// Reference a global constant. Constants are immortal, so the temporary
/// wrapper's release alongside the retained clone is harmless.
unsafe fn constant(value: CFTypeRef) -> CFType {
    unsafe { CFType::wrap_under_get_rule(value).as_CFType() }
}

fn base_query(service: &str, account: &str) -> Vec<(CFType, CFType)> {
    unsafe {
        vec![
            (constant(kSecClass), constant(kSecClassGenericPassword)),
            (
                constant(kSecAttrService),
                CFString::new(service).as_CFType(),
            ),
            (
                constant(kSecAttrAccount),
                CFString::new(account).as_CFType(),
            ),
        ]
    }
}

/// Sibling binaries allowed silent access to our items. Pure over its input
/// so tests never touch the real Keychain.
pub fn companion_binaries(exe_dir: &Path) -> Vec<PathBuf> {
    let suffix = std::env::consts::EXE_SUFFIX;
    [
        "recollect-agent",
        "recollect-mcp-bridge",
        "recollect-mcp-runner",
    ]
    .iter()
    .map(|name| exe_dir.join(format!("{name}{suffix}")))
    .filter(|path| path.is_file())
    .collect()
}

fn trusted_access(label: &str) -> Result<Option<Release>> {
    let exe = std::env::current_exe().map_err(|_| unavailable())?;
    let dir = exe.parent().ok_or_else(unavailable)?;
    let mut apps: Vec<*const c_void> = Vec::new();
    let mut paths = companion_binaries(dir);
    if !paths.iter().any(|path| path == &exe) {
        paths.push(exe);
    }
    for path in paths {
        let Some(text) = path.to_str() else { continue };
        let mut trusted: SecTrustedApplicationRef = ptr::null();
        let status = unsafe {
            SecTrustedApplicationCreateFromPath(
                CFString::new(text).as_concrete_TypeRef(),
                &mut trusted,
            )
        };
        if status == SUCCESS && !trusted.is_null() {
            apps.push(trusted);
        }
    }
    if apps.is_empty() {
        return Ok(None);
    }
    let list = CFArray::from_copyable(&apps);
    let mut access: SecAccessRef = ptr::null();
    let status = unsafe {
        SecAccessCreate(
            CFString::new(label).as_concrete_TypeRef(),
            list.as_concrete_TypeRef(),
            ptr::null(),
            &mut access,
        )
    };
    for app in apps {
        unsafe { CFRelease(app as CFTypeRef) };
    }
    if status != SUCCESS || access.is_null() {
        if !access.is_null() {
            unsafe { CFRelease(access as CFTypeRef) };
        }
        return Err(unavailable());
    }
    Ok(Some(Release(access)))
}

/// Save (or replace, upgrading the access list) one credential.
pub fn save(service: &str, account: &str, secret: &[u8]) -> Result<()> {
    let access = trusted_access(service)?;
    unsafe {
        // Delete first so a re-pair also upgrades items saved without trust.
        let query = CFDictionary::from_CFType_pairs(&base_query(service, account));
        match SecItemDelete(query.as_concrete_TypeRef()) {
            SUCCESS | ITEM_NOT_FOUND => {}
            _ => return Err(unavailable()),
        }
        let mut pairs = base_query(service, account);
        pairs.push((
            constant(kSecValueData),
            CFData::from_buffer(secret).as_CFType(),
        ));
        if let Some(access) = &access {
            pairs.push((
                constant(kSecAttrAccess),
                CFType::wrap_under_create_rule(access.0 as CFTypeRef),
            ));
        }
        let attributes = CFDictionary::from_CFType_pairs(&pairs);
        match SecItemAdd(attributes.as_concrete_TypeRef(), ptr::null_mut()) {
            SUCCESS => Ok(()),
            _ => Err(unavailable()),
        }
    }
}

/// Load one credential, or `None` when this profile was never paired.
pub fn load(service: &str, account: &str) -> Result<Option<Vec<u8>>> {
    unsafe {
        let mut pairs = base_query(service, account);
        pairs.push((
            constant(kSecReturnData),
            CFType::wrap_under_get_rule(kCFBooleanTrue),
        ));
        pairs.push((constant(kSecMatchLimit), constant(kSecMatchLimitOne)));
        let query = CFDictionary::from_CFType_pairs(&pairs);
        let mut result: CFTypeRef = ptr::null();
        match SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) {
            SUCCESS => {
                let data = CFData::wrap_under_create_rule(result as CFDataRef);
                Ok(Some(data.bytes().to_vec()))
            }
            ITEM_NOT_FOUND => Ok(None),
            _ => Err(unavailable()),
        }
    }
}

/// Delete one credential. Reports whether an item existed.
pub fn delete(service: &str, account: &str) -> Result<bool> {
    let query = CFDictionary::from_CFType_pairs(&base_query(service, account));
    match unsafe { SecItemDelete(query.as_concrete_TypeRef()) } {
        SUCCESS => Ok(true),
        ITEM_NOT_FOUND => Ok(false),
        _ => Err(unavailable()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_binaries_lists_only_existing_siblings() {
        let dir = std::env::temp_dir().join(format!(
            "recollect-os-store-fixture-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let agent = dir.join(format!("recollect-agent{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&agent, b"fixture").unwrap();
        let found = companion_binaries(&dir);
        assert_eq!(found, vec![agent]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn companion_binaries_empty_without_siblings() {
        let dir =
            std::env::temp_dir().join(format!("recollect-os-store-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(companion_binaries(&dir).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
