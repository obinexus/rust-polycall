// Compile the libpolycall C core from source with the SAME toolchain Rust uses
// (MSVC on *-pc-windows-msvc, GNU on *-gnu / Linux). This makes rust-polycall a
// self-contained crate that links cleanly everywhere, instead of depending on a
// pre-built static archive that may not match Rust's linker.
use std::path::PathBuf;

fn main() {
    // bindings/rust-polycall -> repo root is two levels up.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let src = root.join("src");
    let include = root.join("include");

    let units = [
        "core/polycall_context.c",
        "core/polycall_core.c",
        "config/polycall_config.c",
        "config/polycall_config_schema.c",
        "runtime/polycall_runtime.c",
        "network/polycall_network.c",
        "protocol/polycall_protocol.c",
        "ffi/polycall_ffi.c",
        "adapter/polycall_adapter.c",
        "diagnostics/polycall_diagnostics.c",
    ];

    let mut build = cc::Build::new();
    build.include(&include).include(&src);
    for u in units {
        let p = src.join(u);
        println!("cargo:rerun-if-changed={}", p.display());
        build.file(p);
    }
    build.compile("polycall"); // emits libpolycall.a / polycall.lib and links it
}
