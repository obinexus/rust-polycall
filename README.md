# rust-polycall

Safe Rust binding for the [Polycall](https://github.com/obinexus/polycall)
C library, **binding ABI v1** (`polycall.h`, library >= 1.1.0).

The crate declares the exact C signatures from `polycall.h` (`rust_polycall::sys`)
and wraps them in a safe API. Configuration parsing, RPC framing and the
`polycall-peer/1` wire protocol all stay in the C core.

| Area | API |
| --- | --- |
| Library | `check_abi()`, `abi_version()`, `version()`, `library_version()`, `strerror()`, `last_error()` |
| Configuration | `run_config(Some(path), run)`, `describe(Some(path))` |
| RPC | `call(endpoint, service, operation, input_json, timeout_ms)` |
| Peer nodes | `Peer::open / close / endpoint / node_id / register / unregister / list / ping / send / recv / recv_with_capacity / cancel / health` (closed on `Drop`) |
| Errors | `Error { code, name, detail, output, needed }` - `name` is `polycall_strerror(code)`, `detail` is `polycall_last_error()` |

Every wrapper first checks `polycall_ffi_abi_version() == 1` and returns an
`E_UNSUPPORTED` error naming both versions otherwise.

## Requirements

* Rust **1.82** or newer (`unsafe extern "C"` blocks).
* An installed Polycall core (>= 1.1.0): the shared library, and either a
  `polycall.pc` on `PKG_CONFIG_PATH` or `POLYCALL_LIB_DIR`.

## Linking (`build.rs`)

1. `POLYCALL_LIB_DIR=<dir>` - link `polycall` from that directory
   (`POLYCALL_LIB_NAME`, `POLYCALL_LINK_KIND=dylib|static` refine it).
2. otherwise `pkg-config --libs polycall` (honours `PKG_CONFIG_PATH`).
3. otherwise `-lpolycall` from the default linker path, with a warning.

This is a link-time binding: the dynamic loader resolves every symbol when
the program starts, so an old 1.0 library without the ABI v1 symbols fails
at load time with the loader's "undefined symbol" message rather than
crashing later. At run time the shared library must be on the loader path
(`LD_LIBRARY_PATH=/opt/polycall/lib` on Linux, `PATH` containing
`polycall.dll` / `libpolycall.dll` on Windows).

```sh
# Linux, core installed under /opt/polycall
export PKG_CONFIG_PATH=/opt/polycall/lib/pkgconfig LD_LIBRARY_PATH=/opt/polycall/lib
cargo run --example demo -- rust-polycallrc
```

## Example

```rust
use rust_polycall::{Peer, WAIT_FOREVER};

rust_polycall::run_config(Some("rust-polycallrc"), true)?;

let inbox = Peer::open("beta", Some("127.0.0.1:0"), Some("shared-token"))?;
let sender = Peer::open("alpha", None, Some("shared-token"))?;
sender.send(&inbox.endpoint()?, b"hello", Some("m-1"), 5000)?;
let msg = inbox.recv(WAIT_FOREVER)?;
assert_eq!((msg.sender.as_str(), msg.payload.as_slice()), ("alpha", &b"hello"[..]));

let out = rust_polycall::call("127.0.0.1:8084", "inventory", "get",
                              Some(r#"{"item_id":"widget-a"}"#), 2000)?;
```

`examples/peer_tool.rs` is a tiny peer process (`serve` / `send`) used by
the cross-language interop tests.

## Tests

`tests/real_core.rs` runs against the **real** installed library and the
`polycall` CLI (`POLYCALL_CLI`, default `polycall` on `PATH`); there are no
mocks. It covers the checklist of the core's `docs/BINDING_ABI.md`: version
and ABI check; `run_config` valid / missing / invalid / strict / unsupported
TLS; `describe`; `call` against `polycall start` (success, unknown operation,
remote error, deadline, invalid input, no runtime); two nodes exchanging in
both directions with sender and message id verified; empty, UTF-8,
binary-with-NUL, exactly 1 MiB and 1 MiB + 1 payloads; registry ownership;
duplicate message id delivered once; auth failure; dead peer
(`E_TRANSPORT`); receive timeout; too-small buffer keeps the message
queued; cancel and close waking a blocked receive; double close, calls after
close and invalid handles; concurrent senders; and interop with a
`polycall peer serve` C node in both directions.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

## npm

`@obinexusltd/rust-polycall` (not yet published) ships this source tree for
JavaScript tooling; `require('@obinexusltd/rust-polycall')` returns an index
of its files. It does not load the native library from JavaScript.

Author: Nnamdi Michael Okpala <okpalan@protonmail.com> - License: MIT
