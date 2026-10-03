# TODO - rust-polycall

Status: safe binding over the Polycall binding ABI v1 (polycall >= 1.1.0),
tested against the real installed core (Linux: rust:1 and the MSRV 1.82).

- [x] Declare the exact ABI v1 signatures from `polycall.h` (`sys`).
- [x] Safe API: version/ABI check, run_config, describe, call, `Peer` (Drop).
- [x] `Error` with status code, `polycall_strerror` name and `polycall_last_error` detail.
- [x] Link through `POLYCALL_LIB_DIR` or pkg-config.
- [x] Integration, loader-error, unit and doc tests against the real library
      and CLI; clippy clean; valgrind and ASan (nightly) clean.
- [x] Windows: cross-compiled `x86_64-pc-windows-gnu` tests pass on Windows
      against `libpolycall.dll` and `polycall.dll`.
- [ ] Run `cargo test` natively on Windows (MSVC target with `polycall.lib`,
      GNU target with `libpolycall.dll.a`) - no Rust toolchain on the QA host.
- [ ] Publish the crate to crates.io (name `rust-polycall` is unclaimed) when a release is planned.
- [ ] Publish `@obinexusltd/rust-polycall` to npm when a release is planned.
