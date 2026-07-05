'use strict';

const fs = require('node:fs');
const path = require('node:path');

const relativeDirectories = Object.freeze({
  src: 'src',
  dist: 'dist',
  examples: 'examples',
  tests: 'tests'
});

function walkFiles(root, current = root) {
  return fs.readdirSync(current, { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => {
      const absolute = path.join(current, entry.name);
      return entry.isDirectory() ? walkFiles(root, absolute) : [absolute];
    });
}

function indexDirectory(relative) {
  const root = path.join(__dirname, relative);
  const files = walkFiles(root);
  return Object.freeze({
    relative,
    root,
    files: Object.freeze(files),
    relativeFiles: Object.freeze(
      files.map((file) => path.relative(root, file).split(path.sep).join('/'))
    )
  });
}

const directories = Object.freeze(Object.fromEntries(
  Object.entries(relativeDirectories).map(([name, relative]) => [name, indexDirectory(relative)])
));

function resolve(directoryName, ...segments) {
  const directory = directories[directoryName];
  if (!directory) {
    throw new RangeError(`unknown rust-polycall directory: ${directoryName}`);
  }
  const resolved = path.resolve(directory.root, ...segments);
  const prefix = `${directory.root}${path.sep}`;
  if (resolved !== directory.root && !resolved.startsWith(prefix)) {
    throw new RangeError(`path escapes rust-polycall ${directoryName} directory`);
  }
  return resolved;
}

module.exports = Object.freeze({
  packageName: '@obinexusltd/rust-polycall',
  projectRoot: __dirname,
  directories,
  resolve,
  cargoManifest: path.join(__dirname, 'Cargo.toml'),
  cargoLock: path.join(__dirname, 'Cargo.lock'),
  buildScript: path.join(__dirname, 'build.rs'),
  rustLibrary: path.join(__dirname, 'src', 'lib.rs'),
  config: path.join(__dirname, 'rust-polycallrc'),
  manifest: path.join(__dirname, 'polycall-binding.json')
});
