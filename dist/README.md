# Distribution notes

The npm artifact contains the complete Rust source binding, Cargo metadata,
examples and tests. It excludes `target/` and all platform binaries; the
crate links the separately installed Polycall C library
(https://github.com/obinexus/polycall, binding ABI v1) at build time.
