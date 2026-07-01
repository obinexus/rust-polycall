//! rust-polycall - Rust reference adapter for libpolycall.
//!
//! A thin, safe wrapper over the flat FFI boundary (`polycall_ffi.h`). No config
//! parsing or runtime logic lives here: every call forwards to the shared core.
//! This is the template for other systems-language bindings.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// Symbols from the shared core (include/polycall/polycall_ffi.h).
extern "C" {
    fn polycall_ffi_version(buf: *mut c_char, buf_len: c_int) -> c_int;
    fn polycall_ffi_run_config(path: *const c_char, run: c_int) -> c_int;
    fn polycall_ffi_describe(path: *const c_char, buf: *mut c_char, buf_len: c_int) -> c_int;
}

/// Error carrying the raw `polycall_status_t` code from the core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolycallError {
    pub code: i32,
    pub message: String,
}

impl std::fmt::Display for PolycallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "polycall error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for PolycallError {}

/// Return the shared core version, e.g. `"1.5.0"`.
pub fn version() -> String {
    let mut buf = [0i8; 64];
    unsafe {
        polycall_ffi_version(buf.as_mut_ptr(), buf.len() as c_int);
        CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
    }
}

/// Load a config (Polycallfile / polycallrc / *polycallrc) and optionally run.
pub fn run_config(path: Option<&str>, run: bool) -> Result<(), PolycallError> {
    let c_path = path.map(|p| CString::new(p).unwrap());
    let ptr = c_path.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    let rc = unsafe { polycall_ffi_run_config(ptr, run as c_int) };
    if rc == 0 {
        Ok(())
    } else {
        Err(PolycallError { code: rc, message: format!("run_config({:?})", path) })
    }
}

/// Load a config and return a one-line human summary (never opens sockets).
pub fn describe(path: Option<&str>) -> Result<String, PolycallError> {
    let mut buf = [0i8; 256];
    let c_path = path.map(|p| CString::new(p).unwrap());
    let ptr = c_path.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    let rc = unsafe { polycall_ffi_describe(ptr, buf.as_mut_ptr(), buf.len() as c_int) };
    let text = unsafe { CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned() };
    if rc == 0 {
        Ok(text)
    } else {
        Err(PolycallError { code: rc, message: text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_1_5_0() {
        assert_eq!(version(), "1.5.0");
    }

    #[test]
    fn run_defaults_ok() {
        run_config(None, true).expect("defaults should run");
    }
}
