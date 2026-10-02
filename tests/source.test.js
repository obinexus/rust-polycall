'use strict';

// Static checks of the Rust source: every binding ABI v1 symbol is declared
// with the signature from polycall.h, linking goes through pkg-config /
// POLYCALL_LIB_DIR, and the adapter stays thin (no protocol or parsing code).

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const read = (file) => fs.readFileSync(path.join(root, file), 'utf8');
const library = read('src/lib.rs');
// rustfmt may wrap declarations: compare on a whitespace-normalised copy
const flat = library.replace(/\s+/g, ' ').replace(/\( /g, '(').replace(/,? \)/g, ')');
const build = read('build.rs');
const cargo = read('Cargo.toml');

const declarations = {
  polycall_ffi_abi_version: /fn polycall_ffi_abi_version\(\) -> c_int;/,
  polycall_ffi_version: /fn polycall_ffi_version\(buf: \*mut c_char, len: c_int\) -> c_int;/,
  polycall_strerror: /fn polycall_strerror\(status: c_int\) -> \*const c_char;/,
  polycall_last_error: /fn polycall_last_error\(buf: \*mut c_char, cap: usize\) -> c_int;/,
  polycall_ffi_run_config: /fn polycall_ffi_run_config\(config_path: \*const c_char, run: c_int\) -> c_int;/,
  polycall_ffi_describe: /fn polycall_ffi_describe\(config_path: \*const c_char, buf: \*mut c_char, len: c_int\)/,
  polycall_call: /fn polycall_call\(/,
  polycall_peer_open: /fn polycall_peer_open\(/,
  polycall_peer_close: /fn polycall_peer_close\(h: polycall_peer_t\) -> c_int;/,
  polycall_peer_endpoint: /fn polycall_peer_endpoint\(/,
  polycall_peer_node_id: /fn polycall_peer_node_id\(/,
  polycall_peer_register: /fn polycall_peer_register\(/,
  polycall_peer_unregister: /fn polycall_peer_unregister\(/,
  polycall_peer_list: /fn polycall_peer_list\(/,
  polycall_peer_ping: /fn polycall_peer_ping\(/,
  polycall_peer_send: /fn polycall_peer_send\(/,
  polycall_peer_recv: /fn polycall_peer_recv\(/,
  polycall_peer_cancel: /fn polycall_peer_cancel\(h: polycall_peer_t\) -> c_int;/,
  polycall_peer_health: /fn polycall_peer_health\(/
};
for (const [symbol, pattern] of Object.entries(declarations)) {
  assert.match(flat, pattern, `missing or wrong declaration of ${symbol}`);
}
assert.match(library, /unsafe extern "C"/);
assert.match(cargo, /rust-version = "1\.82"/, 'unsafe extern blocks need Rust 1.82');
assert.match(build, /POLYCALL_LIB_DIR/);
assert.match(build, /pkg-config/);
assert.match(build, /cargo:rustc-link-lib/);
assert.doesNotMatch(build, /cc::Build/);
assert.doesNotMatch(library, /#\[no_mangle\]/, 'the core must not be mocked inside the crate');

for (const forbidden of ['serde_json', 'reqwest', 'TcpStream', 'std::fs::read_to_string']) {
  assert.ok(!library.includes(forbidden), `adapter must not contain ${forbidden}`);
}

console.log('rust-polycall ABI declaration / thin-adapter source test: PASS');
