//! Access control lists on files and directories, read and written as
//! SDDL strings.
#![allow(unsafe_code)]

use std::io;
use std::path::Path;
use std::ptr;

use widestring::{U16CStr, U16CString};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, LocalFree, WIN32_ERROR};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW, SDDL_REVISION_1,
    SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, UNPROTECTED_DACL_SECURITY_INFORMATION,
};

use crate::sddl;

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
    let descriptor = Descriptor::parse(sddl)?;
    let mut info = DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
    let owner = descriptor.owner()?;
    if !owner.is_null() {
        info |= OWNER_SECURITY_INFORMATION;
    }
    set(path, info, owner, descriptor.dacl()?)
}

/// Reads the DACL of `path` as an SDDL string (`D:...`).
///
/// # Errors
/// If the permissions cannot be read.
pub fn read_dacl(path: &Path) -> io::Result<String> {
    let path = wide_path(path)?;
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    // SAFETY: `path` is NUL-terminated and the out-pointer is a live local.
    // The other out-pointers are optional and passed as null.
    let status = unsafe {
        GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &raw mut descriptor,
        )
    };
    check(status)?;
    let descriptor = Descriptor(descriptor);

    let mut text: *mut u16 = ptr::null_mut();
    // SAFETY: `descriptor` is valid; the out-pointer is a live local.
    let ok = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor.0,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &raw mut text,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: Windows returned a NUL-terminated string allocated with
    // LocalAlloc; it is copied and then freed.
    let sddl = unsafe {
        let sddl = U16CStr::from_ptr_str(text).to_string_lossy();
        LocalFree(text.cast());
        sddl
    };
    Ok(sddl)
}

/// Replaces the DACL of `path` with `sddl` (`D:...`), keeping its
/// inheritance setting as written in `sddl`. Inherited ACEs in `sddl` are
/// ignored; Windows recomputes them from the parent.
///
/// # Errors
/// If the SDDL is invalid or the DACL cannot be set.
pub fn write_dacl(path: &Path, sddl: &str) -> io::Result<()> {
    let descriptor = Descriptor::parse(sddl)?;
    let inheritance = if sddl::blocks_inheritance(sddl) {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    set(
        path,
        DACL_SECURITY_INFORMATION | inheritance,
        ptr::null_mut(),
        descriptor.dacl()?,
    )
}

fn set(
    path: &Path,
    info: OBJECT_SECURITY_INFORMATION,
    owner: PSID,
    dacl: *mut ACL,
) -> io::Result<()> {
    let path = wide_path(path)?;
    // SAFETY: `path` is NUL-terminated; `owner` and `dacl` point into a
    // descriptor the caller keeps alive (or are null). Group and SACL are
    // left unchanged.
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
    check(status)
}

/// A self-relative security descriptor allocated by Windows.
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    fn parse(sddl: &str) -> io::Result<Self> {
        let sddl = U16CString::from_str(sddl).map_err(invalid)?;
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

    /// The DACL, which points into this descriptor.
    fn dacl(&self) -> io::Result<*mut ACL> {
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl: *mut ACL = ptr::null_mut();
        // SAFETY: the descriptor is valid and the out-pointers are live locals.
        let ok = unsafe {
            GetSecurityDescriptorDacl(self.0, &raw mut present, &raw mut dacl, &raw mut defaulted)
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if present == 0 {
            return Err(invalid("the SDDL has no DACL"));
        }
        Ok(dacl)
    }

    /// The owner SID, which points into this descriptor, or null.
    fn owner(&self) -> io::Result<PSID> {
        let mut owner: PSID = ptr::null_mut();
        let mut defaulted = 0;
        // SAFETY: the descriptor is valid and the out-pointers are live locals.
        let ok = unsafe { GetSecurityDescriptorOwner(self.0, &raw mut owner, &raw mut defaulted) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(owner)
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: allocated with LocalAlloc by Windows.
        unsafe { LocalFree(self.0) };
    }
}

fn wide_path(path: &Path) -> io::Result<U16CString> {
    U16CString::from_os_str(path).map_err(invalid)
}

fn check(status: WIN32_ERROR) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status.cast_signed()))
    }
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dacl_round_trips_through_sddl() {
        let dir = tempfile::tempdir().unwrap();
        let before = read_dacl(dir.path()).unwrap();
        assert!(before.starts_with("D:"), "{before}");
        write_dacl(dir.path(), &before).unwrap();
        // Writing may add the auto-inherited flag (`D:AI`); the ACEs stay.
        let aces = |sddl: &str| sddl[sddl.find('(').unwrap()..].to_owned();
        assert_eq!(aces(&read_dacl(dir.path()).unwrap()), aces(&before));
    }
}
