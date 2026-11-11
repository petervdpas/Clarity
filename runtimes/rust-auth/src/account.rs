//! The computer's own account: the name of the user the program runs as,
//! from the system, not from the environment (which anyone can set).

use crate::{Problem, ProblemKind};

#[cfg(unix)]
pub fn system_account() -> Result<String, Problem> {
    let uid = unsafe { libc::getuid() };
    let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buf = vec![0 as libc::c_char; 4096];
    let mut found: *mut libc::passwd = std::ptr::null_mut();
    let code = unsafe { libc::getpwuid_r(uid, &mut entry, buf.as_mut_ptr(), buf.len(), &mut found) };
    if code != 0 || found.is_null() || entry.pw_name.is_null() {
        return Err(Problem::new(ProblemKind::Failed, format!("the system names no account for user {uid}")));
    }
    Ok(unsafe { std::ffi::CStr::from_ptr(entry.pw_name) }.to_string_lossy().into_owned())
}

#[cfg(windows)]
pub fn system_account() -> Result<String, Problem> {
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn GetUserNameW(buffer: *mut u16, size: *mut u32) -> i32;
    }
    let mut buf = vec![0u16; 257];
    let mut size = buf.len() as u32;
    if unsafe { GetUserNameW(buf.as_mut_ptr(), &mut size) } == 0 || size == 0 {
        return Err(Problem::new(ProblemKind::Failed, "the system names no account for this user"));
    }
    Ok(String::from_utf16_lossy(&buf[..size as usize - 1]))
}

#[cfg(not(any(unix, windows)))]
pub fn system_account() -> Result<String, Problem> {
    Err(Problem::new(ProblemKind::Failed, "this system has no accounts Clarity knows"))
}
