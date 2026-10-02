'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const api = require('..');
const pkg = require('../package.json');

assert.equal(pkg.name, '@obinexusltd/rust-polycall');
// one version across the npm, Cargo and binding manifests
const cargoVersion = /^version = "([^"]+)"/m.exec(fs.readFileSync(path.join(__dirname, '..', 'Cargo.toml'), 'utf8'))[1];
assert.equal(pkg.version, cargoVersion);
assert.equal(require('../polycall-binding.json').version, cargoVersion);
assert.equal(pkg.author, 'Nnamdi Michael Okpala <okpalan@protonmail.com>');
assert.equal(pkg.license, 'MIT');
assert.equal(pkg.publishConfig.access, 'public');
assert.equal(api.packageName, pkg.name);

const packageDirectoryKeys = {
  src: 'src',
  dist: 'dist',
  examples: 'example',
  tests: 'test'
};

for (const [apiKey, packageKey] of Object.entries(packageDirectoryKeys)) {
  const directory = api.directories[apiKey];
  assert.equal(directory.relative, pkg.directories[packageKey]);
  assert.ok(path.isAbsolute(directory.root));
  assert.ok(directory.files.length > 0, `${apiKey} must contain files`);
  directory.files.forEach((file) => assert.ok(fs.statSync(file).isFile()));
}

assert.ok(api.directories.src.relativeFiles.includes('lib.rs'));
assert.ok(api.directories.examples.relativeFiles.includes('demo.rs'));
assert.ok(api.directories.tests.relativeFiles.includes('source.test.js'));

for (const file of [api.cargoManifest, api.cargoLock, api.buildScript,
  api.rustLibrary, api.config, api.manifest]) {
  assert.ok(fs.statSync(file).isFile(), `${file} must exist`);
}

assert.throws(() => api.resolve('src', '..', 'package.json'), RangeError);
assert.throws(() => api.resolve('unknown'), RangeError);

console.log('rust-polycall npm package and relative-directory index test: PASS');
