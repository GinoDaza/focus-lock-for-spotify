//! Access control lists on files and directories.
//!
//! This is the only module of the service that calls Win32 directly.
#![allow(unsafe_code)]

use std::io;
use std::path::Path;
use std::ptr;

use widestring::U16CString;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_FILE_OBJECT,
    SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
};

/// Owned by Administrators, with full control for SYSTEM and
/// Administrators, inherited by everything inside, and nothing for anyone
/// else. Setting the owner matters: an owner can always rewrite the DACL.
pub const SYSTEM_AND_ADMINS_ONLY: &str = "O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

/// Replaces the DACL of `path` with the one in `sddl` and stops it from
/// inheriting entries from its parent. Also sets the owner if `sddl` has
/// one.
///
/// # Errors
/// If the SDDL is invalid or the DACL cannot be set.
pub fn set_protected_dacl(path: &Path, sddl: &str) -> io::Result<()> {
    let path = U16CString::from_os_str(path).map_err(invalid)?;
    let sddl = U16CString::from_str(sddl).map_err(invalid)?;
    let descriptor = Descriptor::parse(&sddl)?;

    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl: *mut ACL = ptr::null_mut();
    // SAFETY: `descriptor` is a valid descriptor until dropped, and the
    // out-pointers point to live locals. `dacl` points into `descriptor`.
    let ok = unsafe {
        GetSecurityDescriptorDacl(
            descriptor.0,
            &raw mut present,
            &raw mut dacl,
            &raw mut defaulted,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    if present == 0 {
        return Err(invalid("the SDDL has no DACL"));
    }
    let mut owner: PSID = ptr::null_mut();
    // SAFETY: as above; `owner` points into `descriptor` or stays null.
    let ok =
        unsafe { GetSecurityDescriptorOwner(descriptor.0, &raw mut owner, &raw mut defaulted) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut info = DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
    if !owner.is_null() {
        info |= OWNER_SECURITY_INFORMATION;
    }

    // SAFETY: `path` is NUL-terminated and `dacl` is valid while
    // `descriptor` lives. Group and SACL are left unchanged.
    let status = unsafe {
        SetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            info,
            owner,
            ptr::null_mut(),
            dacl,
            ptr::null(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status.cast_signed()));
    }
    Ok(())
}

/// A self-relative security descriptor allocated by Windows.
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    fn parse(sddl: &U16CString) -> io::Result<Self> {
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        // SAFETY: `sddl` is NUL-terminated and the out-pointer is a live local.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &raw mut descriptor,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(descriptor))
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: allocated with LocalAlloc by the conversion above.
        unsafe { LocalFree(self.0) };
    }
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}
