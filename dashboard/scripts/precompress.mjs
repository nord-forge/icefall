// Post-build step: precompress static assets so the Rust server can serve a
// build-time Brotli (`.br`) variant directly — zero per-request CPU, best ratio
// (brotli quality 11). Important on a 1 vCPU server.
//
// IF-256: we emit ONLY `.br`, not `.gz`. Brotli is accepted by ~all modern
// browsers; the rare gzip-only (or no-Accept-Encoding) client is served the
// identity asset, which the server's CompressionLayer (IF-252) gzips on the fly.
// Dropping the `.gz` copies shrinks the dashboard embedded in the binary (IF-255)
// by ~a third of the compressed set.
//
// Uses node:zlib (built in), so no CLI or npm dependency. Runs after
// `astro build` + csp-hashes via the build script.

import {
  readdirSync,
  statSync,
  readFileSync,
  writeFileSync,
} from 'node:fs';
import { join, extname } from 'node:path';
import {
  brotliCompressSync,
  constants as zlibConstants,
} from 'node:zlib';

const DIST_DIR = join(import.meta.dirname, '..', 'dist');

// Only compress text-like assets — binary formats (images, fonts, already-
// compressed) don't benefit and waste build time + disk.
const COMPRESSIBLE = new Set([
  '.js',
  '.mjs',
  '.css',
  '.html',
  '.json',
  '.svg',
  '.xml',
  '.txt',
  '.map',
]);

// Below this size, compression overhead (headers + a separate request) isn't
// worth it; the browser may as well take the identity file.
const MIN_BYTES = 1024;

const BROTLI_OPTS = {
  params: {
    [zlibConstants.BROTLI_PARAM_QUALITY]: 11, // max
    [zlibConstants.BROTLI_PARAM_SIZE_HINT]: 0, // set per file below
  },
};

/** Recursively collect every file under a directory. */
function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

let compressed = 0;
let originalBytes = 0;
let brBytes = 0;

for (const file of walk(DIST_DIR)) {
  // Never re-compress our own outputs.
  if (file.endsWith('.br') || file.endsWith('.gz')) continue;
  if (!COMPRESSIBLE.has(extname(file))) continue;

  const data = readFileSync(file);
  if (data.length < MIN_BYTES) continue;

  const br = brotliCompressSync(data, {
    params: {
      ...BROTLI_OPTS.params,
      [zlibConstants.BROTLI_PARAM_SIZE_HINT]: data.length,
    },
  });

  // Only keep the variant if it's actually smaller than the original — otherwise
  // the server would hand the client a *bigger* "compressed" file.
  if (br.length < data.length) writeFileSync(`${file}.br`, br);

  compressed += 1;
  originalBytes += data.length;
  brBytes += Math.min(br.length, data.length);
}

const pct =
  originalBytes > 0
    ? Math.round((1 - brBytes / originalBytes) * 100)
    : 0;
console.log(
  `precompress: ${compressed} files -> .br, ` +
    `${Math.round(originalBytes / 1024)} KB -> ${Math.round(brBytes / 1024)} KB brotli (${pct}% smaller)`,
);
