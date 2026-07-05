# @obinexusltd/rust-polycall

Thin, safe Rust source binding for libpolycall 1.5. The crate translates Rust
strings and results at the FFI boundary; configuration parsing, networking,
and runtime behavior remain in the shared C core.

The npm package distributes the complete Cargo source tree and exposes every
project directory through a location-independent CommonJS index.

## Install

From npm:

```bash
npm install @obinexusltd/rust-polycall
```

As a Rust path dependency:

```toml
[dependencies]
rust-polycall = { path = "node_modules/@obinexusltd/rust-polycall" }
```

## Link libpolycall

Set the directory containing `polycall.lib`, `libpolycall.a`, or the shared
library import file before building a final executable:

```powershell
$env:POLYCALL_LIB_DIR = 'C:\path\to\libpolycall\lib'
$env:POLYCALL_LIB_NAME = 'polycall'     # optional; this is the default
$env:POLYCALL_LINK_KIND = 'dylib'       # optional: dylib or static
cargo run --example demo -- rust-polycallrc
```

The crate does not compile or duplicate the C runtime. Unit tests install
small ABI mocks, so `cargo test --lib` needs no local libpolycall build.

## Rust API

```rust
let version = rust_polycall::version()?;
let summary = rust_polycall::describe(Some("rust-polycallrc"))?;
rust_polycall::run_config(Some("rust-polycallrc"), true)?;
```

Core status codes are preserved in `PolycallError::code`. Paths containing an
interior NUL are rejected before crossing the C boundary.

## npm source index

```js
const rustPolycall = require('@obinexusltd/rust-polycall');

console.log(rustPolycall.directories.src.relativeFiles);
console.log(rustPolycall.resolve('examples', 'demo.rs'));
```

`src`, `dist`, `examples`, and `tests` are recursively indexed with both
absolute paths and package-relative filenames. Resolution rejects traversal
outside those roots.

## Verification

```bash
npm test
npm pack --dry-run
```

The mandatory prepack suite runs Rust unit tests, checks the example, validates
npm metadata/directories, and enforces the thin-adapter source boundary.

Author: Nnamdi Michael Okpala <okpalan@protonmail.com>

License: MIT
