# rust-polycall

**Rust** binding for [libpolycall](https://github.com/obinexus/libpolycall) — an
implemented reference adapter.

A thin, safe wrapper over the flat FFI boundary (`polycall_ffi.h`). It contains
no config or runtime logic; every call forwards to the shared C core. See
[../../docs/adapter-pattern.md](../../docs/adapter-pattern.md).

## Self-contained build

`build.rs` compiles the C core from `../../src` with the **same toolchain Rust
uses** (via the `cc` crate), so the crate links cleanly on MSVC, MinGW/GNU, and
Linux without a separate core build step:

```bash
cd bindings/rust-polycall
cargo build
cargo test        # version_is_1_5_0, run_defaults_ok
cargo run --example demo -- rust-polycallrc
```

## API

```rust
rust_polycall::version();                            // "1.5.0"
rust_polycall::run_config(Some("rust-polycallrc"), true)?;
let summary = rust_polycall::describe(None)?;
```

## Config

Read-only config: [`rust-polycallrc`](rust-polycallrc) — the standard
`*polycallrc` convention on the single shared schema. No per-language parser.

## Manifest

See [`polycall-binding.json`](polycall-binding.json).
