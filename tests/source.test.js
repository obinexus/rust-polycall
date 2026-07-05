'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const read = (file) => fs.readFileSync(path.join(root, file), 'utf8');
const library = read('src/lib.rs');
const build = read('build.rs');

for (const symbol of ['polycall_ffi_version', 'polycall_ffi_run_config', 'polycall_ffi_describe']) {
  assert.ok(library.includes(symbol), `missing FFI symbol ${symbol}`);
}
assert.match(library, /CString::new/);
assert.match(library, /CStr::from_ptr/);
assert.match(build, /POLYCALL_LIB_DIR/);
assert.match(build, /cargo:rustc-link-search/);
assert.match(build, /cargo:rustc-link-lib/);
assert.doesNotMatch(build, /cc::Build|\.join\("\.\."\).*\.join\("\.\."\)/s);

for (const forbidden of ['serde_json', 'reqwest', 'TcpStream', 'std::fs::read_to_string']) {
  assert.ok(!library.includes(forbidden), `adapter must not contain ${forbidden}`);
}

console.log('rust-polycall thin-adapter source test: PASS');
