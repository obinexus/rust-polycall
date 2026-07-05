//! Safe Rust calls over libpolycall's flat C FFI.
//!
//! This crate owns conversion and error mapping only. Configuration parsing,
//! networking, and runtime behavior remain in libpolycall.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString, NulError};
use std::os::raw::{c_char, c_int};

const VERSION_CAPACITY: usize = 64;
const DESCRIPTION_CAPACITY: usize = 256;
const INVALID_RUST_INPUT: i32 = -1;

unsafe extern "C" {
    fn polycall_ffi_version(buffer: *mut c_char, buffer_length: c_int) -> c_int;
    fn polycall_ffi_run_config(path: *const c_char, run: c_int) -> c_int;
    fn polycall_ffi_describe(
        path: *const c_char,
        buffer: *mut c_char,
        buffer_length: c_int,
    ) -> c_int;
}

/// A libpolycall status together with adapter context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolycallError {
    pub code: i32,
    pub message: String,
}

impl PolycallError {
    fn from_nul(error: NulError) -> Self {
        Self {
            code: INVALID_RUST_INPUT,
            message: format!(
                "path contains an interior NUL byte at {}",
                error.nul_position()
            ),
        }
    }
}

impl std::fmt::Display for PolycallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "polycall error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for PolycallError {}

fn path_pointer(path: Option<&str>) -> Result<(Option<CString>, *const c_char), PolycallError> {
    let owned = path
        .map(CString::new)
        .transpose()
        .map_err(PolycallError::from_nul)?;
    let pointer = owned
        .as_ref()
        .map_or(std::ptr::null(), |value| value.as_ptr());
    Ok((owned, pointer))
}

fn buffer_text(buffer: &[c_char]) -> String {
    // Every adapter buffer is zero-initialized with one byte kept outside the
    // length passed to C, so a terminator is always available.
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

/// Return the shared libpolycall version.
pub fn version() -> Result<String, PolycallError> {
    let mut buffer = vec![0 as c_char; VERSION_CAPACITY + 1];
    let status = unsafe { polycall_ffi_version(buffer.as_mut_ptr(), VERSION_CAPACITY as c_int) };
    let text = buffer_text(&buffer);
    if status == 0 {
        Ok(text)
    } else {
        Err(PolycallError {
            code: status,
            message: if text.is_empty() {
                "version".to_owned()
            } else {
                text
            },
        })
    }
}

/// Load a config and optionally execute it, preserving the core status code.
pub fn run_config(path: Option<&str>, run: bool) -> Result<(), PolycallError> {
    let (_path, pointer) = path_pointer(path)?;
    let status = unsafe { polycall_ffi_run_config(pointer, c_int::from(run)) };
    if status == 0 {
        Ok(())
    } else {
        Err(PolycallError {
            code: status,
            message: format!("run_config({path:?})"),
        })
    }
}

/// Return libpolycall's one-line description without duplicating core logic.
pub fn describe(path: Option<&str>) -> Result<String, PolycallError> {
    let (_path, pointer) = path_pointer(path)?;
    let mut buffer = vec![0 as c_char; DESCRIPTION_CAPACITY + 1];
    let status = unsafe {
        polycall_ffi_describe(pointer, buffer.as_mut_ptr(), DESCRIPTION_CAPACITY as c_int)
    };
    let text = buffer_text(&buffer);
    if status == 0 {
        Ok(text)
    } else {
        Err(PolycallError {
            code: status,
            message: if text.is_empty() {
                "describe".to_owned()
            } else {
                text
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    unsafe fn write_c_string(destination: *mut c_char, capacity: c_int, value: &str) {
        assert!(!destination.is_null());
        let bytes = value.as_bytes();
        assert!(bytes.len() < capacity as usize);
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), destination.cast::<u8>(), bytes.len());
            destination.add(bytes.len()).write(0);
        }
    }

    #[no_mangle]
    unsafe extern "C" fn polycall_ffi_version(buffer: *mut c_char, buffer_length: c_int) -> c_int {
        unsafe { write_c_string(buffer, buffer_length, "1.5.0") };
        0
    }

    #[no_mangle]
    unsafe extern "C" fn polycall_ffi_run_config(path: *const c_char, run: c_int) -> c_int {
        assert_eq!(run, 1);
        if path.is_null() {
            return 0;
        }
        let path = unsafe { CStr::from_ptr(path) }.to_string_lossy();
        if path == "__status_37__" {
            37
        } else {
            0
        }
    }

    #[no_mangle]
    unsafe extern "C" fn polycall_ffi_describe(
        path: *const c_char,
        buffer: *mut c_char,
        buffer_length: c_int,
    ) -> c_int {
        let text = if path.is_null() {
            "libpolycall defaults"
        } else {
            "libpolycall configured"
        };
        unsafe { write_c_string(buffer, buffer_length, text) };
        0
    }

    #[test]
    fn version_comes_from_the_core() {
        assert_eq!(version().unwrap(), "1.5.0");
    }

    #[test]
    fn run_config_forwards_defaults_and_status() {
        run_config(None, true).unwrap();
        run_config(Some("rust-polycallrc"), true).unwrap();
        assert_eq!(
            run_config(Some("__status_37__"), true).unwrap_err().code,
            37
        );
    }

    #[test]
    fn interior_nul_is_a_rust_input_error() {
        assert_eq!(
            run_config(Some("bad\0path"), true).unwrap_err().code,
            INVALID_RUST_INPUT
        );
    }

    #[test]
    fn describe_forwards_to_the_core() {
        assert_eq!(describe(None).unwrap(), "libpolycall defaults");
        assert_eq!(
            describe(Some("rust-polycallrc")).unwrap(),
            "libpolycall configured"
        );
    }
}
