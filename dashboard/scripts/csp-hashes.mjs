// Post-build step: scan the static build for inline <script> blocks and
// emit their SHA-256 hashes to dist/csp-hashes.json.
//
// The Rust server reads that file at startup and builds a strict
// `script-src` CSP directive from it (see src/api/middleware.rs). This lets
// the dashboard keep Astro's inline hydration bootstrap scripts WITHOUT
// `unsafe-inline` in the CSP. Styles still use `unsafe-inline` by decision
// (Astro scoped styles, low XSS risk) so they are not hashed here.
//
// Run automatically after `astro build` via the `build` npm script.

import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const DIST_DIR = join(import.meta.dirname, '..', 'dist');

/** Recursively collect every .html file under a directory. */
function htmlFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...htmlFiles(full));
    else if (entry.endsWith('.html')) out.push(full);
  }
  return out;
}

// Match <script> blocks that have NO src attribute (i.e. inline code).
const INLINE_SCRIPT = /<script(?![^>]*\ssrc=)[^>]*>([\s\S]*?)<\/script>/g;

const hashes = new Set();
let scriptCount = 0;

for (const file of htmlFiles(DIST_DIR)) {
  const html = readFileSync(file, 'utf-8');
  for (const match of html.matchAll(INLINE_SCRIPT)) {
    scriptCount += 1;
    const digest = createHash('sha256').update(match[1], 'utf-8').digest('base64');
    hashes.add(`sha256-${digest}`);
  }
}

const sorted = [...hashes].sort();
writeFileSync(
  join(DIST_DIR, 'csp-hashes.json'),
  JSON.stringify(sorted, null, 2) + '\n',
);

console.log(
  `csp-hashes: ${sorted.length} unique inline-script hashes from ${scriptCount} scripts -> dist/csp-hashes.json`,
);
