// Builds the engine (crates/wasm) for the browser into src/wasm/pkg/.
//
//   1. cargo compiles the crate to WebAssembly,
//   2. wasm-bindgen writes the JavaScript glue around it.
//
// The wasm-bindgen command-line tool has to be exactly the version the crate
// uses (see Cargo.lock), or the glue and the engine do not fit together; this
// script checks that and says how to install the right one.

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const webDir = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoDir = join(webDir, '..');
const outDir = join(webDir, 'src', 'wasm', 'pkg');

function run(command, args, options = {}) {
  return execFileSync(command, args, { stdio: 'inherit', ...options });
}

const lock = readFileSync(join(repoDir, 'Cargo.lock'), 'utf8');
const wanted = /name = "wasm-bindgen"\r?\nversion = "([^"]+)"/.exec(lock)?.[1];
if (!wanted) {
  throw new Error('wasm-bindgen is not in Cargo.lock.');
}

let installed = '';
try {
  installed = execFileSync('wasm-bindgen', ['--version'], { encoding: 'utf8' }).trim();
} catch {
  // handled below
}
if (!installed.endsWith(` ${wanted}`)) {
  console.error(
    `The wasm-bindgen tool must be version ${wanted} (found: ${installed || 'none'}).\n` +
      `Install it with:  cargo install wasm-bindgen-cli --version ${wanted} --locked`,
  );
  process.exit(1);
}

run('cargo', ['build', '-p', 'img2ico-wasm', '--target', 'wasm32-unknown-unknown', '--release', '--locked'], {
  cwd: repoDir,
});
run('wasm-bindgen', [
  join(repoDir, 'target', 'wasm32-unknown-unknown', 'release', 'img2ico_wasm.wasm'),
  '--target', 'web',
  '--out-dir', outDir,
  '--out-name', 'img2ico_wasm',
]);
