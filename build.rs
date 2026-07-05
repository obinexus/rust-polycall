//! Configure linking to an existing libpolycall build.
//!
//! The adapter deliberately does not compile or copy the shared C core.

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=POLYCALL_LIB_DIR");
    println!("cargo:rerun-if-env-changed=POLYCALL_LIB_NAME");
    println!("cargo:rerun-if-env-changed=POLYCALL_LINK_KIND");

    let Ok(directory) = env::var("POLYCALL_LIB_DIR") else {
        println!(
            "cargo:warning=POLYCALL_LIB_DIR is unset; the final executable must provide libpolycall"
        );
        return;
    };

    let name = env::var("POLYCALL_LIB_NAME").unwrap_or_else(|_| "polycall".to_owned());
    let kind = env::var("POLYCALL_LINK_KIND").unwrap_or_else(|_| "dylib".to_owned());
    assert!(
        matches!(kind.as_str(), "dylib" | "static"),
        "POLYCALL_LINK_KIND must be dylib or static"
    );

    println!("cargo:rustc-link-search=native={directory}");
    println!("cargo:rustc-link-lib={kind}={name}");
}
