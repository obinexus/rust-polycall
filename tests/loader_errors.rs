//! Loading failures (Linux): this crate is a link-time binding, so the
//! dynamic loader resolves libpolycall when a program starts. Rust links
//! with full RELRO (`-z now`), so every symbol is bound at start-up and a
//! missing library or an old 1.0 library without the ABI v1 symbols stops
//! the program before `main` with the loader's message (exit 127) -- never
//! a crash on first use. A library that has the symbols but reports another
//! ABI version is refused by `check_abi()` with `E_UNSUPPORTED`.
//!
//! Each case re-runs THIS test binary as a child process with
//! `LD_LIBRARY_PATH` pointing at the library under test, running only the
//! ignored `loader_child` test.
//!
//! * missing library: an empty directory
//! * old library: the REAL polycall 1.0.0 library named by
//!   `POLYCALL_TEST_OLD_LIBRARY` (built from the core's v1.0.0 commit)
//! * ABI mismatch: a stand-in exporting every ABI v1 symbol, whose
//!   `polycall_ffi_abi_version()` returns 2 (compiled here with `cc`)
//!
//! A missing prerequisite (no `cc`, no old library) fails when
//! `POLYCALL_REQUIRE_LOADER_TESTS=1`, otherwise the case is reported as
//! skipped on stderr -- never as a pass of that case.
#![cfg(target_os = "linux")]

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CHILD_ENV: &str = "RUST_POLYCALL_LOADER_CHILD";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rust-polycall-loader-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_child(lib_dir: &Path, mode: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "loader_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("LD_LIBRARY_PATH", lib_dir)
        .env(CHILD_ENV, mode)
        .output()
        .unwrap()
}

fn skip_or_fail(what: &str) {
    if std::env::var("POLYCALL_REQUIRE_LOADER_TESTS").as_deref() == Ok("1") {
        panic!("{what} (required by POLYCALL_REQUIRE_LOADER_TESTS=1)");
    }
    eprintln!("SKIP: {what}");
}

fn describe(out: &Output) -> String {
    format!(
        "status {:?} signal {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        out.status.signal(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Runs only inside the child process (see the module docs).
#[test]
#[ignore = "child process of the loader tests"]
fn loader_child() {
    let mode = std::env::var(CHILD_ENV).unwrap_or_default();
    // reaching this line at all means the loader accepted the library
    println!("CHILD-STARTED {mode}");
    let e = rust_polycall::check_abi().expect_err("ABI 2 must be refused");
    assert_eq!(e.code, rust_polycall::status::UNSUPPORTED);
    println!("CHILD-REFUSED {}", e.detail);
    // every wrapper refuses the library the same way -- no call reaches it
    assert_eq!(rust_polycall::version().unwrap_err().code, e.code);
    assert_eq!(
        rust_polycall::run_config(Some("x"), true).unwrap_err().code,
        e.code
    );
    assert_eq!(
        rust_polycall::Peer::open("n", None, None).unwrap_err().code,
        e.code
    );
    assert_eq!(
        rust_polycall::call("127.0.0.1:1", "a", "b", None, 1000)
            .unwrap_err()
            .code,
        e.code
    );
    println!("CHILD-OK");
}

#[test]
fn missing_library_is_a_loader_error_not_a_crash() {
    let empty = scratch("missing");
    let out = run_child(&empty, "missing");
    let text = describe(&out);
    if out.status.success() || String::from_utf8_lossy(&out.stdout).contains("CHILD-STARTED") {
        skip_or_fail("libpolycall is reachable without LD_LIBRARY_PATH (system install); cannot simulate a missing library");
        return;
    }
    assert_eq!(out.status.signal(), None, "{text}");
    assert_eq!(out.status.code(), Some(127), "{text}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("libpolycall.so.1") && stderr.contains("cannot open shared object file"),
        "{text}"
    );
}

#[test]
fn old_library_without_abi_v1_stops_at_startup() {
    let Ok(old) = std::env::var("POLYCALL_TEST_OLD_LIBRARY") else {
        skip_or_fail("POLYCALL_TEST_OLD_LIBRARY does not name a polycall 1.0 library");
        return;
    };
    if !Path::new(&old).is_file() {
        skip_or_fail(&format!("POLYCALL_TEST_OLD_LIBRARY={old} is not a file"));
        return;
    }
    let dir = scratch("old");
    std::fs::copy(&old, dir.join("libpolycall.so.1")).unwrap();
    let out = run_child(&dir, "old");
    let text = describe(&out);
    assert_eq!(out.status.signal(), None, "{text}");
    assert_eq!(out.status.code(), Some(127), "{text}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("undefined symbol: polycall_"),
        "loader must name the missing ABI v1 symbol: {text}"
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("CHILD-STARTED"),
        "must fail before main, not on first use: {text}"
    );
}

const FAKE_ABI2: &str = r#"
#include <stddef.h>
static const char *S = "POLYCALL_E_UNSUPPORTED: not supported by this build";
int polycall_ffi_abi_version(void) { return 2; }
int polycall_ffi_version(char *b, int n) { if (b && n > 5) { b[0]='9'; b[1]='.'; b[2]='9'; b[3]=0; } return 3; }
const char *polycall_strerror(int s) { (void)s; return S; }
int polycall_last_error(char *b, size_t n) { if (b && n) b[0] = 0; return 0; }
const char *polycall_get_version(void) { return "9.9.9"; }
#define STUB(name) int name() { return -15; }
STUB(polycall_ffi_run_config) STUB(polycall_ffi_describe) STUB(polycall_call)
STUB(polycall_peer_open) STUB(polycall_peer_close) STUB(polycall_peer_endpoint)
STUB(polycall_peer_node_id) STUB(polycall_peer_register) STUB(polycall_peer_unregister)
STUB(polycall_peer_list) STUB(polycall_peer_ping) STUB(polycall_peer_send)
STUB(polycall_peer_recv) STUB(polycall_peer_cancel) STUB(polycall_peer_health)
"#;

#[test]
fn abi_mismatch_is_refused_by_check_abi() {
    let dir = scratch("abi2");
    let src = dir.join("fake.c");
    std::fs::write(&src, FAKE_ABI2).unwrap();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let built = Command::new(&cc)
        .args(["-shared", "-fPIC", "-Wl,-soname,libpolycall.so.1", "-o"])
        .arg(dir.join("libpolycall.so.1"))
        .arg(&src)
        .status();
    match built {
        Ok(s) if s.success() => {}
        other => {
            skip_or_fail(&format!(
                "cannot build the ABI-2 stand-in with {cc}: {other:?}"
            ));
            return;
        }
    }
    let out = run_child(&dir, "abi2");
    let text = describe(&out);
    assert!(out.status.success(), "{text}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CHILD-STARTED abi2"), "{text}");
    assert!(
        stdout.contains(
            "CHILD-REFUSED libpolycall reports binding ABI 2; rust-polycall requires ABI 1"
        ),
        "{text}"
    );
    assert!(stdout.contains("CHILD-OK"), "{text}");
}
