//! Safe Rust binding for the Polycall **binding ABI v1** (`polycall.h`).
//!
//! The crate links the installed C library (`libpolycall.so.1`,
//! `polycall.dll` / `libpolycall.dll`) found through `pkg-config polycall`
//! or `POLYCALL_LIB_DIR` (see `build.rs`) and wraps every ABI v1 entry point:
//!
//! * library: [`abi_version`], [`version`], [`library_version`],
//!   [`strerror`], [`last_error`]
//! * configuration: [`run_config`], [`describe`]
//! * RPC: [`call`]
//! * peer nodes: [`Peer`] (open / close / endpoint / node id / register /
//!   unregister / list / ping / send / recv / cancel / health)
//!
//! Every fallible call returns [`Error`], which carries the status code, its
//! name from `polycall_strerror()` and the per-thread detail from
//! `polycall_last_error()`. Configuration parsing, networking and the wire
//! protocol all stay in the C core; this crate only converts arguments and
//! results. The raw declarations are available in [`sys`].
//!
//! Before the first real call the crate checks
//! `polycall_ffi_abi_version() == 1` and refuses to proceed otherwise.
//!
//! # Example
//!
//! Two nodes in one process exchanging a payload through the real library
//! (each node owns its registry and inbox; nothing is shared implicitly):
//!
//! ```
//! use rust_polycall::{Peer, WAIT_FOREVER};
//!
//! rust_polycall::check_abi()?;
//! let inbox = Peer::open("doc-inbox", Some("127.0.0.1:0"), Some("doc-token"))?;
//! let sender = Peer::open("doc-sender", None, Some("doc-token"))?; // send-only
//! sender.send(&inbox.endpoint()?, b"hello\0world", Some("doc-1"), 5000)?;
//! let msg = inbox.recv(WAIT_FOREVER)?;
//! assert_eq!(msg.sender, "doc-sender");
//! assert_eq!(msg.message_id, "doc-1");
//! assert_eq!(msg.payload, b"hello\0world");
//! # Ok::<(), rust_polycall::Error>(())
//! ```

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Raw declarations of the binding ABI v1, exactly as in `polycall.h`.
///
/// Prefer the safe wrappers; these are exposed for callers that need the
/// C surface itself (for example to test invalid handles).
#[allow(non_camel_case_types, missing_docs)]
pub mod sys {
    use std::os::raw::{c_char, c_int, c_void};

    /// `polycall_peer_t`: a peer handle, `> 0` when valid.
    pub type polycall_peer_t = i32;

    /// `POLYCALL_FFI_ABI_VERSION` this crate was written for.
    pub const POLYCALL_FFI_ABI_VERSION: c_int = 1;

    /// success
    pub const POLYCALL_OK: c_int = 0;
    /// NULL/empty/malformed argument
    pub const POLYCALL_E_INVALID_ARGUMENT: c_int = -1;
    /// allocation failed
    pub const POLYCALL_E_NO_MEMORY: c_int = -2;
    /// unknown, closed or stale peer handle
    pub const POLYCALL_E_INVALID_HANDLE: c_int = -3;
    /// deadline exceeded
    pub const POLYCALL_E_TIMEOUT: c_int = -4;
    /// refused / reset / unreachable
    pub const POLYCALL_E_TRANSPORT: c_int = -5;
    /// malformed reply or wrong identity
    pub const POLYCALL_E_PROTOCOL: c_int = -6;
    /// unknown peer id, file or operation
    pub const POLYCALL_E_NOT_FOUND: c_int = -7;
    /// shared token missing or wrong
    pub const POLYCALL_E_AUTH: c_int = -8;
    /// the remote side reported an error
    pub const POLYCALL_E_REMOTE: c_int = -9;
    /// over a limit, or caller buffer too small
    pub const POLYCALL_E_TOO_LARGE: c_int = -10;
    /// backpressure
    pub const POLYCALL_E_BUSY: c_int = -11;
    /// `polycall_peer_cancel()` woke a blocked receive
    pub const POLYCALL_E_CANCELLED: c_int = -12;
    /// invalid configuration
    pub const POLYCALL_E_CONFIG: c_int = -13;
    /// bind failed: address in use
    pub const POLYCALL_E_ADDRESS_IN_USE: c_int = -14;
    /// valid but not supported by this build
    pub const POLYCALL_E_UNSUPPORTED: c_int = -15;
    /// the OS refused
    pub const POLYCALL_E_PERMISSION: c_int = -16;
    /// the handle was closed while the call was blocked
    pub const POLYCALL_E_CLOSED: c_int = -17;
    /// internal error
    pub const POLYCALL_E_INTERNAL: c_int = -18;

    /// Buffer size for a peer id, including the NUL.
    pub const POLYCALL_PEER_ID_MAX: usize = 64;
    /// Buffer size for a message id, including the NUL.
    pub const POLYCALL_MESSAGE_ID_MAX: usize = 64;
    /// Buffer size for a `host:port` endpoint, including the NUL.
    pub const POLYCALL_ENDPOINT_MAX: usize = 128;
    /// Largest peer payload (1 MiB).
    pub const POLYCALL_PEER_MAX_PAYLOAD: usize = 1 << 20;
    /// Largest `polycall_call` output (1 MiB).
    pub const POLYCALL_CALL_MAX_OUTPUT: usize = 1 << 20;

    unsafe extern "C" {
        pub fn polycall_ffi_abi_version() -> c_int;
        pub fn polycall_ffi_version(buf: *mut c_char, len: c_int) -> c_int;
        pub fn polycall_strerror(status: c_int) -> *const c_char;
        pub fn polycall_last_error(buf: *mut c_char, cap: usize) -> c_int;
        pub fn polycall_get_version() -> *const c_char;

        pub fn polycall_ffi_run_config(config_path: *const c_char, run: c_int) -> c_int;
        pub fn polycall_ffi_describe(
            config_path: *const c_char,
            buf: *mut c_char,
            len: c_int,
        ) -> c_int;

        pub fn polycall_call(
            endpoint: *const c_char,
            service: *const c_char,
            operation: *const c_char,
            input_json: *const c_char,
            timeout_ms: u32,
            out: *mut c_char,
            out_cap: usize,
            out_len: *mut usize,
        ) -> c_int;

        pub fn polycall_peer_open(
            node_id: *const c_char,
            bind_endpoint: *const c_char,
            auth_token: *const c_char,
            out_handle: *mut polycall_peer_t,
        ) -> c_int;
        pub fn polycall_peer_close(h: polycall_peer_t) -> c_int;
        pub fn polycall_peer_endpoint(h: polycall_peer_t, buf: *mut c_char, cap: usize) -> c_int;
        pub fn polycall_peer_node_id(h: polycall_peer_t, buf: *mut c_char, cap: usize) -> c_int;
        pub fn polycall_peer_register(
            h: polycall_peer_t,
            peer_id: *const c_char,
            endpoint: *const c_char,
        ) -> c_int;
        pub fn polycall_peer_unregister(h: polycall_peer_t, peer_id: *const c_char) -> c_int;
        pub fn polycall_peer_list(
            h: polycall_peer_t,
            buf: *mut c_char,
            cap: usize,
            out_len: *mut usize,
        ) -> c_int;
        pub fn polycall_peer_ping(
            h: polycall_peer_t,
            peer: *const c_char,
            timeout_ms: u32,
        ) -> c_int;
        pub fn polycall_peer_send(
            h: polycall_peer_t,
            peer: *const c_char,
            payload: *const c_void,
            len: usize,
            message_id: *const c_char,
            timeout_ms: u32,
        ) -> c_int;
        pub fn polycall_peer_recv(
            h: polycall_peer_t,
            timeout_ms: u32,
            sender: *mut c_char,
            sender_cap: usize,
            message_id: *mut c_char,
            message_id_cap: usize,
            payload: *mut c_void,
            payload_cap: usize,
            payload_len: *mut usize,
        ) -> c_int;
        pub fn polycall_peer_cancel(h: polycall_peer_t) -> c_int;
        pub fn polycall_peer_health(
            h: polycall_peer_t,
            buf: *mut c_char,
            cap: usize,
            out_len: *mut usize,
        ) -> c_int;
    }
}

/// The binding ABI version this crate implements.
pub const ABI_VERSION: i32 = sys::POLYCALL_FFI_ABI_VERSION;
/// Largest peer payload (1 MiB).
pub const PEER_MAX_PAYLOAD: usize = sys::POLYCALL_PEER_MAX_PAYLOAD;
/// Largest `call` output (1 MiB).
pub const CALL_MAX_OUTPUT: usize = sys::POLYCALL_CALL_MAX_OUTPUT;
/// `recv` timeout meaning "wait until a message, cancel or close".
pub const WAIT_FOREVER: u32 = u32::MAX;

/// Status codes of the binding ABI (`POLYCALL_E_*`).
pub mod status {
    pub use super::sys::{
        POLYCALL_E_ADDRESS_IN_USE as ADDRESS_IN_USE, POLYCALL_E_AUTH as AUTH,
        POLYCALL_E_BUSY as BUSY, POLYCALL_E_CANCELLED as CANCELLED, POLYCALL_E_CLOSED as CLOSED,
        POLYCALL_E_CONFIG as CONFIG, POLYCALL_E_INTERNAL as INTERNAL,
        POLYCALL_E_INVALID_ARGUMENT as INVALID_ARGUMENT,
        POLYCALL_E_INVALID_HANDLE as INVALID_HANDLE, POLYCALL_E_NOT_FOUND as NOT_FOUND,
        POLYCALL_E_NO_MEMORY as NO_MEMORY, POLYCALL_E_PERMISSION as PERMISSION,
        POLYCALL_E_PROTOCOL as PROTOCOL, POLYCALL_E_REMOTE as REMOTE,
        POLYCALL_E_TIMEOUT as TIMEOUT, POLYCALL_E_TOO_LARGE as TOO_LARGE,
        POLYCALL_E_TRANSPORT as TRANSPORT, POLYCALL_E_UNSUPPORTED as UNSUPPORTED,
        POLYCALL_OK as OK,
    };
}

/// A failed Polycall call: status code, its name and the core's detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// The `POLYCALL_E_*` status (negative).
    pub code: i32,
    /// `polycall_strerror(code)`, e.g. `"POLYCALL_E_TIMEOUT: deadline exceeded"`.
    pub name: String,
    /// `polycall_last_error()` captured on the failing thread (may be empty).
    pub detail: String,
    /// For [`call`]: the remote error object `{"code":..,"message":..}` when
    /// the core returned one.
    pub output: Option<String>,
    /// For `E_TOO_LARGE` from an output buffer: the size the core needed.
    pub needed: Option<usize>,
}

/// Alias kept for code written against earlier releases of this crate.
pub type PolycallError = Error;

/// `Result` with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Build an error for `code` from the core's own name and the calling
    /// thread's detail message. Call it right after the failing call.
    fn from_core(code: c_int) -> Self {
        Self {
            code,
            name: strerror(code),
            detail: last_error(),
            output: None,
            needed: None,
        }
    }

    /// An error raised by this crate before reaching the core.
    fn local(code: c_int, detail: impl Into<String>) -> Self {
        Self {
            code,
            name: strerror(code),
            detail: detail.into(),
            output: None,
            needed: None,
        }
    }

    /// The short symbolic name (`"POLYCALL_E_TIMEOUT"`).
    pub fn short_name(&self) -> &str {
        self.name.split(':').next().unwrap_or(&self.name)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.code)?;
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

fn cstring(value: &str, what: &str) -> Result<CString> {
    CString::new(value).map_err(|e| {
        Error::local(
            sys::POLYCALL_E_INVALID_ARGUMENT,
            format!(
                "{what} contains an interior NUL byte at {}",
                e.nul_position()
            ),
        )
    })
}

fn opt_cstring(value: Option<&str>, what: &str) -> Result<Option<CString>> {
    value.map(|v| cstring(v, what)).transpose()
}

fn opt_ptr(value: &Option<CString>) -> *const c_char {
    value.as_ref().map_or(std::ptr::null(), |v| v.as_ptr())
}

/// Text up to the first NUL of a caller-owned buffer.
fn buffer_text(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn static_text(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: the core documents these strings as static and NUL-terminated.
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// `polycall_strerror(code)`: the static name of any status code.
///
/// ```
/// use rust_polycall::{status, strerror};
/// assert!(strerror(status::TIMEOUT).starts_with("POLYCALL_E_TIMEOUT"));
/// assert!(strerror(-12345).starts_with("POLYCALL_E_UNKNOWN"));
/// ```
pub fn strerror(code: i32) -> String {
    // SAFETY: plain call; the result is static and never NULL.
    static_text(unsafe { sys::polycall_strerror(code) })
}

/// `polycall_last_error()`: detail of this thread's most recent failure.
pub fn last_error() -> String {
    let mut buf = vec![0u8; 1024];
    // SAFETY: buf is caller-owned with the capacity passed.
    let n = unsafe { sys::polycall_last_error(buf.as_mut_ptr().cast(), buf.len()) };
    if n >= 0 && (n as usize) >= buf.len() {
        buf = vec![0u8; n as usize + 1];
        // SAFETY: as above, with a buffer of the size the core asked for.
        unsafe { sys::polycall_last_error(buf.as_mut_ptr().cast(), buf.len()) };
    }
    buffer_text(&buf)
}

/// `polycall_ffi_abi_version()` of the loaded library (no check).
pub fn abi_version() -> i32 {
    // SAFETY: no arguments.
    unsafe { sys::polycall_ffi_abi_version() }
}

static ABI_CHECK: OnceLock<std::result::Result<(), Error>> = OnceLock::new();

/// Verify, once, that the linked library implements binding ABI v1.
///
/// Every other wrapper calls this first; it is public so applications can
/// fail fast at start-up.
pub fn check_abi() -> Result<()> {
    ABI_CHECK
        .get_or_init(|| {
            let found = abi_version();
            if found == ABI_VERSION {
                Ok(())
            } else {
                Err(Error {
                    code: sys::POLYCALL_E_UNSUPPORTED,
                    name: strerror(sys::POLYCALL_E_UNSUPPORTED),
                    detail: format!(
                        "libpolycall reports binding ABI {found}; rust-polycall requires ABI {ABI_VERSION}"
                    ),
                    output: None,
                    needed: None,
                })
            }
        })
        .clone()
}

/// `polycall_ffi_version()`: the library version (e.g. `"1.1.0"`).
pub fn version() -> Result<String> {
    check_abi()?;
    let mut buf = vec![0u8; 64];
    loop {
        // SAFETY: buf is caller-owned; len is its capacity.
        let n = unsafe { sys::polycall_ffi_version(buf.as_mut_ptr().cast(), buf.len() as c_int) };
        if n < 0 {
            return Err(Error::from_core(n));
        }
        if (n as usize) < buf.len() {
            return Ok(buffer_text(&buf));
        }
        buf = vec![0u8; n as usize + 1];
    }
}

/// `polycall_get_version()` (the 1.0 API, kept by the core).
pub fn library_version() -> Result<String> {
    check_abi()?;
    // SAFETY: returns a static string.
    Ok(static_text(unsafe { sys::polycall_get_version() }))
}

/// Validate one configuration file with the grammar of `polycall config
/// validate` (`polycall_ffi_run_config`).
///
/// `run == false`: validate (unknown keys are warnings). `run == true`:
/// validate for running with this build (unknown keys are errors,
/// `tls_enabled=true` is `E_UNSUPPORTED`). It never starts a service.
/// `None` is passed to the core as NULL (`E_INVALID_ARGUMENT`).
///
/// ```
/// use rust_polycall::{run_config, status};
/// let dir = std::env::temp_dir().join(format!("rp-doc-{}", std::process::id()));
/// std::fs::create_dir_all(&dir).unwrap();
/// let rc = dir.join("rust-polycallrc");
/// std::fs::write(&rc, "log_level=info\nmystery_key=1\n").unwrap();
/// let rc = rc.to_str().unwrap();
/// run_config(Some(rc), false)?; // validate: unknown keys are warnings
/// let err = run_config(Some(rc), true).unwrap_err(); // for running: errors
/// assert_eq!(err.code, status::CONFIG);
/// assert!(err.detail.contains("mystery_key"), "{err}");
/// let err = run_config(Some("/no/such/rust-polycallrc"), true).unwrap_err();
/// assert_eq!(err.short_name(), "POLYCALL_E_NOT_FOUND");
/// # Ok::<(), rust_polycall::Error>(())
/// ```
pub fn run_config(path: Option<&str>, run: bool) -> Result<()> {
    check_abi()?;
    let path = opt_cstring(path, "config path")?;
    // SAFETY: the pointer is NULL or a live NUL-terminated string.
    let rc = unsafe { sys::polycall_ffi_run_config(opt_ptr(&path), c_int::from(run)) };
    if rc == sys::POLYCALL_OK {
        Ok(())
    } else {
        Err(Error::from_core(rc))
    }
}

/// `polycall_ffi_describe()`: a JSON description of a configuration file
/// (layer, servers, peers, key/values; secrets are never resolved).
pub fn describe(path: Option<&str>) -> Result<String> {
    check_abi()?;
    let path = opt_cstring(path, "config path")?;
    let mut buf = vec![0u8; 4096];
    loop {
        // SAFETY: path is NULL or live; buf is caller-owned with its capacity.
        let n = unsafe {
            sys::polycall_ffi_describe(opt_ptr(&path), buf.as_mut_ptr().cast(), buf.len() as c_int)
        };
        if n < 0 {
            return Err(Error::from_core(n));
        }
        if (n as usize) < buf.len() {
            return Ok(buffer_text(&buf));
        }
        if n as usize >= i32::MAX as usize {
            return Err(Error::local(
                sys::POLYCALL_E_TOO_LARGE,
                "description too large",
            ));
        }
        buf = vec![0u8; n as usize + 1];
    }
}

/// One `polycall_rpc` v1 round trip (`polycall_call`) to a runtime started
/// with `polycall start` / `polycall daemon start`.
///
/// Returns the operation's output JSON. On a remote failure the error's
/// [`Error::output`] holds the error object. Executes once; never retried.
/// `timeout_ms` must be 1..=600000.
///
/// ```
/// use rust_polycall::{call, status};
/// // nothing listens on port 1: definitely not delivered
/// let err = call("127.0.0.1:1", "debug", "echo", Some("{}"), 1000).unwrap_err();
/// assert_eq!(err.code, status::TRANSPORT);
/// // invalid JSON input is refused before any I/O
/// let err = call("127.0.0.1:1", "debug", "echo", Some("{oops"), 1000).unwrap_err();
/// assert_eq!(err.code, status::INVALID_ARGUMENT);
/// ```
pub fn call(
    endpoint: &str,
    service: &str,
    operation: &str,
    input_json: Option<&str>,
    timeout_ms: u32,
) -> Result<String> {
    check_abi()?;
    let endpoint = cstring(endpoint, "endpoint")?;
    let service = cstring(service, "service")?;
    let operation = cstring(operation, "operation")?;
    let input = opt_cstring(input_json, "input_json")?;
    // A too-small buffer discards a result that already ran: size for the max.
    let mut out = vec![0u8; CALL_MAX_OUTPUT + 1];
    let mut out_len: usize = 0;
    // SAFETY: every pointer is live for the call; out is caller-owned.
    let rc = unsafe {
        sys::polycall_call(
            endpoint.as_ptr(),
            service.as_ptr(),
            operation.as_ptr(),
            opt_ptr(&input),
            timeout_ms,
            out.as_mut_ptr().cast(),
            out.len(),
            &mut out_len,
        )
    };
    if rc == sys::POLYCALL_OK {
        return Ok(buffer_text(&out));
    }
    let mut err = Error::from_core(rc);
    let text = buffer_text(&out);
    if !text.is_empty() {
        err.output = Some(text);
    }
    if rc == sys::POLYCALL_E_TOO_LARGE {
        err.needed = Some(out_len);
    }
    Err(err)
}

/// A message taken from a node's inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The sender's node id, as asserted by the sender.
    pub sender: String,
    /// The message id (the receiver de-duplicates on `(sender, id)`).
    pub message_id: String,
    /// The payload bytes, exactly as sent.
    pub payload: Vec<u8>,
}

/// A peer node (`polycall_peer_open`). Closed on drop.
///
/// The registry and inbox belong to this node only. All methods are
/// thread-safe; `Peer` is `Send + Sync`, so a receive blocked on one thread
/// can be woken by [`Peer::cancel`] or [`Peer::close`] from another.
#[derive(Debug)]
pub struct Peer {
    handle: sys::polycall_peer_t,
    closed: AtomicBool,
}

fn status(rc: c_int) -> Result<()> {
    if rc == sys::POLYCALL_OK {
        Ok(())
    } else {
        Err(Error::from_core(rc))
    }
}

impl Peer {
    /// Open a node named `node_id`. `bind`: `Some("127.0.0.1:0")` listens on
    /// an ephemeral port, `None` makes a send-only node. `auth_token`:
    /// `None`/`Some("")` = no authentication.
    pub fn open(node_id: &str, bind: Option<&str>, auth_token: Option<&str>) -> Result<Peer> {
        check_abi()?;
        let node_id = cstring(node_id, "node_id")?;
        let bind = opt_cstring(bind, "bind endpoint")?;
        let token = opt_cstring(auth_token, "auth token")?;
        let mut handle: sys::polycall_peer_t = 0;
        // SAFETY: pointers are live or NULL; handle is a valid out pointer.
        let rc = unsafe {
            sys::polycall_peer_open(
                node_id.as_ptr(),
                opt_ptr(&bind),
                opt_ptr(&token),
                &mut handle,
            )
        };
        status(rc)?;
        Ok(Peer {
            handle,
            closed: AtomicBool::new(false),
        })
    }

    /// The raw `polycall_peer_t` handle.
    pub fn handle(&self) -> i32 {
        self.handle
    }

    /// `polycall_peer_close`: stop the listener and wake blocked receivers
    /// (`E_CLOSED`). A second close returns `E_INVALID_HANDLE`.
    pub fn close(&self) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);
        // SAFETY: integer handle, validated by the core.
        status(unsafe { sys::polycall_peer_close(self.handle) })
    }

    fn text(&self, f: unsafe extern "C" fn(i32, *mut c_char, usize) -> c_int) -> Result<String> {
        let mut buf = vec![0u8; sys::POLYCALL_ENDPOINT_MAX];
        // SAFETY: buf is caller-owned with its capacity.
        let rc = unsafe { f(self.handle, buf.as_mut_ptr().cast(), buf.len()) };
        if rc < 0 {
            return Err(Error::from_core(rc));
        }
        Ok(buffer_text(&buf))
    }

    /// The bound `host:port` (`""` for a send-only node).
    pub fn endpoint(&self) -> Result<String> {
        self.text(sys::polycall_peer_endpoint)
    }

    /// This node's id.
    pub fn node_id(&self) -> Result<String> {
        self.text(sys::polycall_peer_node_id)
    }

    /// Add or replace `peer_id -> endpoint` in THIS node's registry.
    pub fn register(&self, peer_id: &str, endpoint: &str) -> Result<()> {
        let id = cstring(peer_id, "peer_id")?;
        let ep = cstring(endpoint, "endpoint")?;
        // SAFETY: live strings.
        status(unsafe { sys::polycall_peer_register(self.handle, id.as_ptr(), ep.as_ptr()) })
    }

    /// Remove `peer_id` from THIS node's registry (`E_NOT_FOUND` if absent).
    pub fn unregister(&self, peer_id: &str) -> Result<()> {
        let id = cstring(peer_id, "peer_id")?;
        // SAFETY: live string.
        status(unsafe { sys::polycall_peer_unregister(self.handle, id.as_ptr()) })
    }

    fn json(
        &self,
        f: unsafe extern "C" fn(i32, *mut c_char, usize, *mut usize) -> c_int,
    ) -> Result<String> {
        let mut buf = vec![0u8; 8192];
        loop {
            let mut need: usize = 0;
            // SAFETY: buf is caller-owned with its capacity; need is valid.
            let rc = unsafe { f(self.handle, buf.as_mut_ptr().cast(), buf.len(), &mut need) };
            if rc == sys::POLYCALL_OK {
                return Ok(buffer_text(&buf));
            }
            if rc == sys::POLYCALL_E_TOO_LARGE && need >= buf.len() {
                buf = vec![0u8; need + 1];
                continue;
            }
            return Err(Error::from_core(rc));
        }
    }

    /// THIS node's registry as a JSON object `{"id":"host:port",...}`.
    pub fn list(&self) -> Result<String> {
        self.json(sys::polycall_peer_list)
    }

    /// This node's health as JSON.
    pub fn health(&self) -> Result<String> {
        self.json(sys::polycall_peer_health)
    }

    /// `GET /health` on `peer` (registered id or `host:port`).
    pub fn ping(&self, peer: &str, timeout_ms: u32) -> Result<()> {
        let peer = cstring(peer, "peer")?;
        // SAFETY: live string.
        status(unsafe { sys::polycall_peer_ping(self.handle, peer.as_ptr(), timeout_ms) })
    }

    /// Deliver `payload` to `peer` (registered id or `host:port`), exactly one
    /// attempt. `Ok` means the receiver stored and acknowledged it. Retrying
    /// with the same `message_id` is safe (the receiver de-duplicates);
    /// `None` lets the core generate an id.
    pub fn send(
        &self,
        peer: &str,
        payload: &[u8],
        message_id: Option<&str>,
        timeout_ms: u32,
    ) -> Result<()> {
        let peer = cstring(peer, "peer")?;
        let id = opt_cstring(message_id, "message_id")?;
        let ptr = if payload.is_empty() {
            std::ptr::null()
        } else {
            payload.as_ptr().cast()
        };
        // SAFETY: payload is a live (ptr, len) pair; strings are live or NULL.
        status(unsafe {
            sys::polycall_peer_send(
                self.handle,
                peer.as_ptr(),
                ptr,
                payload.len(),
                opt_ptr(&id),
                timeout_ms,
            )
        })
    }

    /// Take the oldest message, waiting up to `timeout_ms` (0 = poll,
    /// [`WAIT_FOREVER`] = until a message, cancel or close). Room is made for
    /// the largest legal payload.
    pub fn recv(&self, timeout_ms: u32) -> Result<Message> {
        self.recv_with_capacity(timeout_ms, PEER_MAX_PAYLOAD)
    }

    /// Like [`Peer::recv`] with a payload buffer of `capacity` bytes. If the
    /// message is larger the call fails with `E_TOO_LARGE`, [`Error::needed`]
    /// holds its size, and the message stays queued.
    pub fn recv_with_capacity(&self, timeout_ms: u32, capacity: usize) -> Result<Message> {
        let mut sender = [0u8; sys::POLYCALL_PEER_ID_MAX];
        let mut mid = [0u8; sys::POLYCALL_MESSAGE_ID_MAX];
        let mut payload = vec![0u8; capacity.max(1)];
        let mut len: usize = 0;
        // SAFETY: every buffer is caller-owned with the capacity passed.
        let rc = unsafe {
            sys::polycall_peer_recv(
                self.handle,
                timeout_ms,
                sender.as_mut_ptr().cast(),
                sender.len(),
                mid.as_mut_ptr().cast(),
                mid.len(),
                payload.as_mut_ptr().cast(),
                capacity,
                &mut len,
            )
        };
        if rc != sys::POLYCALL_OK {
            let mut err = Error::from_core(rc);
            if rc == sys::POLYCALL_E_TOO_LARGE {
                err.needed = Some(len);
            }
            return Err(err);
        }
        payload.truncate(len);
        Ok(Message {
            sender: buffer_text(&sender),
            message_id: buffer_text(&mid),
            payload,
        })
    }

    /// Wake every receive currently blocked on this node with `E_CANCELLED`.
    pub fn cancel(&self) -> Result<()> {
        // SAFETY: integer handle.
        status(unsafe { sys::polycall_peer_cancel(self.handle) })
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            // SAFETY: integer handle; a stale handle is defined (E_INVALID_HANDLE).
            unsafe {
                sys::polycall_peer_close(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of the conversion helpers (these link the real library
    //! too: `strerror` is the core's own table).
    use super::*;

    #[test]
    fn buffer_text_stops_at_the_first_nul() {
        assert_eq!(buffer_text(b"abc\0def"), "abc");
        assert_eq!(buffer_text(b"no nul"), "no nul");
        assert_eq!(buffer_text(b""), "");
        assert_eq!(buffer_text(b"\xff\0"), "\u{fffd}");
    }

    #[test]
    fn interior_nul_is_refused_before_the_core() {
        let e = cstring("a\0b", "node_id").unwrap_err();
        assert_eq!(e.code, sys::POLYCALL_E_INVALID_ARGUMENT);
        assert!(e.detail.contains("node_id") && e.detail.contains("at 1"));
        assert!(opt_cstring(None, "x").unwrap().is_none());
        assert!(opt_ptr(&None).is_null());
    }

    #[test]
    fn error_display_and_short_name() {
        let e = Error::local(sys::POLYCALL_E_TIMEOUT, "slow");
        assert_eq!(e.short_name(), "POLYCALL_E_TIMEOUT");
        assert_eq!(e.to_string(), format!("{} (-4): slow", strerror(-4)));
        let quiet = Error::local(sys::POLYCALL_E_BUSY, "");
        assert_eq!(quiet.to_string(), format!("{} (-11)", strerror(-11)));
        let as_dyn: Box<dyn std::error::Error> = Box::new(e.clone());
        assert!(as_dyn.to_string().contains("slow"));
    }

    #[test]
    fn status_codes_match_polycall_h() {
        let codes = [
            (status::OK, 0, "POLYCALL_OK"),
            (status::INVALID_ARGUMENT, -1, "POLYCALL_E_INVALID_ARGUMENT"),
            (status::NO_MEMORY, -2, "POLYCALL_E_NO_MEMORY"),
            (status::INVALID_HANDLE, -3, "POLYCALL_E_INVALID_HANDLE"),
            (status::TIMEOUT, -4, "POLYCALL_E_TIMEOUT"),
            (status::TRANSPORT, -5, "POLYCALL_E_TRANSPORT"),
            (status::PROTOCOL, -6, "POLYCALL_E_PROTOCOL"),
            (status::NOT_FOUND, -7, "POLYCALL_E_NOT_FOUND"),
            (status::AUTH, -8, "POLYCALL_E_AUTH"),
            (status::REMOTE, -9, "POLYCALL_E_REMOTE"),
            (status::TOO_LARGE, -10, "POLYCALL_E_TOO_LARGE"),
            (status::BUSY, -11, "POLYCALL_E_BUSY"),
            (status::CANCELLED, -12, "POLYCALL_E_CANCELLED"),
            (status::CONFIG, -13, "POLYCALL_E_CONFIG"),
            (status::ADDRESS_IN_USE, -14, "POLYCALL_E_ADDRESS_IN_USE"),
            (status::UNSUPPORTED, -15, "POLYCALL_E_UNSUPPORTED"),
            (status::PERMISSION, -16, "POLYCALL_E_PERMISSION"),
            (status::CLOSED, -17, "POLYCALL_E_CLOSED"),
            (status::INTERNAL, -18, "POLYCALL_E_INTERNAL"),
        ];
        for (constant, value, name) in codes {
            assert_eq!(constant, value, "{name}");
            // the core's strerror agrees on every name
            assert!(
                strerror(value).starts_with(name),
                "{name}: {}",
                strerror(value)
            );
        }
    }

    #[test]
    fn limits_match_polycall_h() {
        assert_eq!(sys::POLYCALL_PEER_ID_MAX, 64);
        assert_eq!(sys::POLYCALL_MESSAGE_ID_MAX, 64);
        assert_eq!(sys::POLYCALL_ENDPOINT_MAX, 128);
        assert_eq!(PEER_MAX_PAYLOAD, 1 << 20);
        assert_eq!(CALL_MAX_OUTPUT, 1 << 20);
        assert_eq!(WAIT_FOREVER, u32::MAX);
        assert_eq!(ABI_VERSION, 1);
    }
}
