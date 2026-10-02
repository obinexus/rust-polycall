//! Link the installed Polycall C library (binding ABI v1).
//!
//! Resolution order (no hard-coded paths, nothing is compiled or copied):
//!
//! 1. `POLYCALL_LIB_DIR` -- a directory holding the import/shared library
//!    (`libpolycall.so`, `polycall.lib`, `libpolycall.dll.a`, ...).
//!    `POLYCALL_LIB_NAME` (default `polycall`) and `POLYCALL_LINK_KIND`
//!    (`dylib` default, or `static`) refine it.
//! 2. `pkg-config polycall` (honours `PKG_CONFIG_PATH`; `PKG_CONFIG` selects
//!    the pkg-config executable).
//! 3. Otherwise `-lpolycall` from the linker's default search path, with a
//!    warning.
//!
//! A static link also needs the core's private dependencies (`-lws2_32` on
//! Windows, `-lpthread` elsewhere); pkg-config `--static` supplies them.

use std::env;
use std::process::Command;

fn emit_pkg_config(kind: &str) -> bool {
    let tool = env::var("PKG_CONFIG").unwrap_or_else(|_| "pkg-config".to_owned());
    let mut cmd = Command::new(&tool);
    cmd.arg("--libs");
    if kind == "static" {
        cmd.arg("--static");
    }
    cmd.arg("polycall");
    let output = match cmd.output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            println!(
                "cargo:warning=`{tool} --libs polycall` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return false;
        }
        Err(error) => {
            println!("cargo:warning=cannot run {tool}: {error}");
            return false;
        }
    };
    let libs = String::from_utf8_lossy(&output.stdout);
    let mut linked = false;
    for token in libs.split_whitespace() {
        if let Some(dir) = token.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={dir}");
        } else if let Some(lib) = token.strip_prefix("-l") {
            if lib == "polycall" {
                println!("cargo:rustc-link-lib={kind}=polycall");
                linked = true;
            } else {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
    }
    linked
}

fn main() {
    for var in [
        "POLYCALL_LIB_DIR",
        "POLYCALL_LIB_NAME",
        "POLYCALL_LINK_KIND",
        "PKG_CONFIG",
        "PKG_CONFIG_PATH",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let kind = env::var("POLYCALL_LINK_KIND").unwrap_or_else(|_| "dylib".to_owned());
    assert!(
        matches!(kind.as_str(), "dylib" | "static"),
        "POLYCALL_LINK_KIND must be dylib or static"
    );
    let windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");

    if let Ok(directory) = env::var("POLYCALL_LIB_DIR") {
        let name = env::var("POLYCALL_LIB_NAME").unwrap_or_else(|_| "polycall".to_owned());
        println!("cargo:rustc-link-search=native={directory}");
        println!("cargo:rustc-link-lib={kind}={name}");
        if kind == "static" {
            println!(
                "cargo:rustc-link-lib={}",
                if windows { "ws2_32" } else { "pthread" }
            );
        }
        return;
    }

    if emit_pkg_config(&kind) {
        return;
    }

    println!(
        "cargo:warning=libpolycall not located via POLYCALL_LIB_DIR or pkg-config; \
         linking -lpolycall from the default search path"
    );
    println!("cargo:rustc-link-lib={kind}=polycall");
}
