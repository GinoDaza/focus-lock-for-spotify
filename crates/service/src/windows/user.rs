//! Acting on behalf of the user logged on at the console: finding their
//! folders and starting processes in their session.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::Duration;

use widestring::{U16CStr, U16CString};
use windows_sys::Win32::Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows_sys::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW, GetExitCodeProcess,
    PROCESS_INFORMATION, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{
    FOLDERID_LocalAppData, FOLDERID_RoamingAppData, KF_FLAG_DONT_VERIFY, SHGetKnownFolderPath,
};
use windows_sys::core::GUID;

/// The primary token of the user logged on at the physical console, or
/// `None` if nobody is. Only works for a service running as `LocalSystem`.
#[must_use]
pub fn console_user() -> Option<OwnedHandle> {
    // SAFETY: no arguments.
    let session = unsafe { WTSGetActiveConsoleSessionId() };
    if session == u32::MAX {
        return None;
    }
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: the out-pointer is a live local. Fails when no user is
    // logged on to the session.
    let ok = unsafe { WTSQueryUserToken(session, &raw mut token) };
    // SAFETY: on success Windows hands over a token handle we now own.
    (ok != 0).then(|| unsafe { OwnedHandle::from_raw_handle(token) })
}

/// The user's roaming application data folder (`%APPDATA%`).
///
/// # Errors
/// If the folder cannot be resolved.
pub fn roaming_app_data(token: &OwnedHandle) -> io::Result<PathBuf> {
    known_folder(token, &FOLDERID_RoamingAppData)
}

/// The user's local application data folder (`%LOCALAPPDATA%`).
///
/// # Errors
/// If the folder cannot be resolved.
pub fn local_app_data(token: &OwnedHandle) -> io::Result<PathBuf> {
    known_folder(token, &FOLDERID_LocalAppData)
}

fn known_folder(token: &OwnedHandle, id: &GUID) -> io::Result<PathBuf> {
    let mut path: *mut u16 = ptr::null_mut();
    // SAFETY: `id` and the out-pointer are valid; the token is a live handle.
    let result = unsafe {
        SHGetKnownFolderPath(
            id,
            KF_FLAG_DONT_VERIFY.cast_unsigned(),
            token.as_raw_handle(),
            &raw mut path,
        )
    };
    // SAFETY: the returned string (or null on failure) must be freed with
    // CoTaskMemFree in both cases; on success it is NUL-terminated.
    unsafe {
        let folder =
            (result >= 0).then(|| PathBuf::from(U16CStr::from_ptr_str(path).to_os_string()));
        CoTaskMemFree(path.cast());
        folder.ok_or_else(|| io::Error::from_raw_os_error(result))
    }
}

/// Runs `program` with `args` as the user owning `token`, in their
/// session and environment, with no window. Waits up to `timeout` and
/// returns the exit code; a process that takes longer is terminated.
///
/// # Errors
/// If the process cannot be started or times out.
pub fn run_as(
    token: &OwnedHandle,
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> io::Result<u32> {
    let application = U16CString::from_os_str(program).map_err(invalid)?;
    let mut command_line = format!("\"{}\"", program.display());
    for arg in args {
        command_line.push(' ');
        command_line.push_str(arg);
    }
    // CreateProcessW may write into the command line buffer.
    let mut command_line = U16CString::from_str(command_line)
        .map_err(invalid)?
        .into_vec_with_nul();
    let directory = program
        .parent()
        .map(U16CString::from_os_str)
        .transpose()
        .map_err(invalid)?;
    let mut desktop = U16CString::from_str("winsta0\\default")
        .map_err(invalid)?
        .into_vec_with_nul();

    let environment = Environment::new(token)?;
    let startup = STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(u32::MAX),
        lpDesktop: desktop.as_mut_ptr(),
        // SAFETY: STARTUPINFOW is plain data; all-zero is its documented
        // default for the remaining fields.
        ..unsafe { std::mem::zeroed() }
    };
    // SAFETY: plain data, filled in by CreateProcessAsUserW.
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: every pointer refers to a live, NUL-terminated buffer or
    // struct above, or is null where the API allows it.
    let ok = unsafe {
        CreateProcessAsUserW(
            token.as_raw_handle(),
            application.as_ptr(),
            command_line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
            environment.0,
            directory.as_ref().map_or(ptr::null(), |d| d.as_ptr()),
            &raw const startup,
            &raw mut info,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: on success both handles are ours to close.
    let (process, _thread) = unsafe {
        (
            OwnedHandle::from_raw_handle(info.hProcess),
            OwnedHandle::from_raw_handle(info.hThread),
        )
    };

    let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
    // SAFETY: `process` is a live process handle.
    match unsafe { WaitForSingleObject(process.as_raw_handle(), millis) } {
        WAIT_OBJECT_0 => {}
        WAIT_TIMEOUT => {
            // SAFETY: as above.
            unsafe { TerminateProcess(process.as_raw_handle(), 1) };
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the process did not finish in time",
            ));
        }
        _ => return Err(io::Error::last_os_error()),
    }
    let mut code = 0;
    // SAFETY: `process` is live and the out-pointer is a live local.
    if unsafe { GetExitCodeProcess(process.as_raw_handle(), &raw mut code) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(code)
}

/// The user's environment block, as their logon would build it.
struct Environment(*mut c_void);

impl Environment {
    fn new(token: &OwnedHandle) -> io::Result<Self> {
        let mut block: *mut c_void = ptr::null_mut();
        // SAFETY: the out-pointer is a live local; the token is live.
        if unsafe { CreateEnvironmentBlock(&raw mut block, token.as_raw_handle(), 0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(block))
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        // SAFETY: created by CreateEnvironmentBlock.
        unsafe { DestroyEnvironmentBlock(self.0) };
    }
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}
