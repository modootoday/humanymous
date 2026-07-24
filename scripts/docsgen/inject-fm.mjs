// inject-fm.mjs — merge the docs-seo-panel per-page metadata (seo.json) into each
// page's Jekyll front matter as `description:` + `keywords:`, so {% seo %} (meta +
// OpenGraph) and the custom per-page JSON-LD render UNIQUE, optimized values instead
// of the site-wide fallback. Idempotent: re-running updates the two keys in place and
// leaves everything else (title, body) untouched. Run AFTER any body edits.

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';

const ROOT = resolve(dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..', '..');
const DOCS = join(ROOT, 'docs');
const SEO = JSON.parse(readFileSync(new URL('./seo.json', import.meta.url)));

let n = 0, miss = 0;
for (const [url, meta] of Object.entries(SEO)) {
  if (!meta.description || !meta.keywords) continue;
  const rel = url === '/' ? 'README.md' : url.replace(/^\//, '').replace(/\.html$/, '.md');
  const file = join(DOCS, rel);
  if (!existsSync(file)) { console.log('  missing:', rel); miss++; continue; }

  let raw = readFileSync(file, 'utf8');
  const descLine = `description: ${JSON.stringify(meta.description)}`;
  const kwLine = `keywords: ${JSON.stringify(meta.keywords)}`;
  const m = raw.match(/^---\r?\n([\s\S]*?)\r?\n?---\r?\n?/);
  if (m) {
    // Keep existing front-matter lines except a prior description/keywords; append ours.
    const keep = m[1].split(/\r?\n/).filter((l) => l.trim() !== '' && !/^(description|keywords):/.test(l));
    raw = `---\n${[...keep, descLine, kwLine].join('\n')}\n---\n` + raw.slice(m[0].length).replace(/^\r?\n/, '');
    raw = raw.replace(/^(---\n[\s\S]*?\n---\n)/, '$1\n'); // one blank line after front matter
  } else {
    raw = `---\n${descLine}\n${kwLine}\n---\n\n` + raw;
  }
  writeFileSync(file, raw);
  n++;
}
console.log(`inject-fm: description + keywords set on ${n} pages${miss ? ` (${miss} missing)` : ''}`);
