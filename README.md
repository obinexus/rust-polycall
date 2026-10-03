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
the program starts (Rust links Linux programs with `-z now`; Windows binds
DLL imports at load time), so a missing library, or an old 1.0 library
without the ABI v1 symbols, stops the program before `main` with the
loader's message (Linux: exit 127, "cannot open shared object file" /
"undefined symbol: polycall_..."), never a crash on first use. A library
that reports another binding ABI is refused by `check_abi()` with
`E_UNSUPPORTED`. At run time the shared library must be on the loader path
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
and ABI check; error code + `polycall_strerror` name + per-thread
`polycall_last_error` detail; `run_config` valid / missing / invalid /
strict / unsupported TLS and a non-ASCII path; `describe`; `call` against
`polycall start` and `polycall daemon start` (success, unknown operation,
remote error, deadline, invalid input, concurrent calls, authenticated
`stop`, no runtime); two nodes exchanging in both directions with sender
and message id verified; empty, UTF-8, binary-with-NUL, exactly 1 MiB and
1 MiB + 1 payloads; registry ownership; duplicate message id delivered
once; auth failure; dead peer (`E_TRANSPORT`); receive timeout; too-small
buffer keeps the message queued; integer boundaries (deadlines, buffer
capacities, id lengths); cancel and close waking a blocked receive while
other threads keep working; double close, calls after close and invalid
handles; concurrent senders; and interop with a `polycall peer serve` C node
in both directions. Only documented CLI arguments are used.

`tests/loader_errors.rs` (Linux) re-runs its own test binary against an
empty directory, the real polycall 1.0.0 library (`POLYCALL_TEST_OLD_LIBRARY`)
and an ABI-2 stand-in built with `cc`; `POLYCALL_REQUIRE_LOADER_TESTS=1`
turns a missing prerequisite into a failure. Its `loader_child` test is
`#[ignore]`d: it is the child entry point those tests run. The crate's unit
tests and doc examples also run against the real library.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

Windows: no Rust toolchain was available on the QA host, so `cargo test`
has not run natively there. The test executables cross-compiled for
`x86_64-pc-windows-gnu` (linking `libpolycall.dll.a` or the MSVC
`polycall.lib`) pass when run on Windows against `libpolycall.dll` and
`polycall.dll`.

## npm

`rust-polycall` (not yet published) ships this source tree for
JavaScript tooling; `require('rust-polycall')` returns an index
of its files. It does not load the native library from JavaScript.

Author: Nnamdi Michael Okpala <okpalan@protonmail.com> - License: MIT
