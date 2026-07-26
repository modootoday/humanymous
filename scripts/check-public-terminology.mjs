// Guard reader-facing prose against project-private coordinate labels.
// Exact identifiers remain allowed inside code spans/fences and compatibility
// examples, as required by docs/style-guide.md.

import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
} from 'node:fs';
import { extname, join, relative, resolve } from 'node:path';

const root = resolve(process.argv[2] || new URL('..', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
const failures = [];

const excluded = new Set([
  'docs/AGENTS.md',
  'docs/reference/versioning-derivation-index.md',
]);

const privateCoordinates = [
  { label: 'internal specification number', pattern: /\bSoT-\d+\b/gi },
  { label: 'internal plan number', pattern: /\bPLAN-\d+\b/gi },
  { label: 'numbered enforcement-rule identifier', pattern: /\bHR-(?:\d+|\*)\b/gi },
  { label: 'numbered detection-stage range', pattern: /\bL[1-7]\s*(?:[-–—]|→)\s*L?[1-7]\b/gi },
  { label: 'numbered detection-stage code', pattern: /\bL[1-7]\b/g },
  { label: 'numbered threat-tier range', pattern: /\bT[0-7]\s*(?:[-–—]|→)\s*T?[0-7]\b/gi },
  { label: 'numbered threat-tier code', pattern: /\bT[0-7]\b/g },
  { label: 'raw signal identifier', pattern: /\bl[1-7]\.[a-z0-9_.-]+\b/gi },
];

function walk(path) {
  if (!existsSync(path)) return [];
  if (!statSync(path).isDirectory()) return [path];
  return readdirSync(path).flatMap((name) => walk(join(path, name)));
}

function stripMarkdownCode(text) {
  return text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/^(?: {0,3})(`{3,}|~{3,})[^\r\n]*\r?\n[\s\S]*?^(?: {0,3})\1[ \t]*$/gm, '')
    .replace(/`[^`\r\n]*`/g, '')
    .replace(/\[[^\]]*]\([^)]*\)/g, (link) => link.replace(/\]\([^)]*\)$/, ']'));
}

function stripHtmlImplementation(text) {
  return text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '')
    .replace(/<[^>]+>/g, ' ');
}

function report(path, text, checks) {
  const rel = relative(root, path).replaceAll('\\', '/');
  const lines = text.split(/\r?\n/);
  for (const { label, pattern } of checks) {
    for (let index = 0; index < lines.length; index += 1) {
      pattern.lastIndex = 0;
      const match = pattern.exec(lines[index]);
      if (match) failures.push(`${rel}:${index + 1}: ${label}: ${match[0]}`);
    }
  }
}

const markdownRoots = [
  'README.md',
  'SECURITY.md',
  'CONTRIBUTING.md',
  'api',
  'docs',
  'deployments',
].map((path) => join(root, path));

for (const path of markdownRoots.flatMap(walk)) {
  const rel = relative(root, path).replaceAll('\\', '/');
  if (extname(path).toLowerCase() !== '.md' || excluded.has(rel)) continue;
  const prose = stripMarkdownCode(readFileSync(path, 'utf8'));
  report(path, prose, privateCoordinates);
}

const generatedRoot = join(root, 'docs', 'assets', 'llms');
for (const path of walk(generatedRoot)) {
  if (!path.endsWith('.llms.txt')) continue;
  const prose = stripMarkdownCode(readFileSync(path, 'utf8'));
  report(path, prose, privateCoordinates);
}

for (const rel of [
  'internal/gate/console.html',
  'web/index.html',
  'web/demo.html',
  'web/playground.html',
  'web/pass.html',
]) {
  const path = join(root, rel);
  if (existsSync(path)) report(path, stripHtmlImplementation(readFileSync(path, 'utf8')), privateCoordinates);
}

for (const rel of ['docs/_data/nav.yml', 'docs/llms.txt']) {
  const path = join(root, rel);
  if (existsSync(path)) report(path, stripMarkdownCode(readFileSync(path, 'utf8')), privateCoordinates);
}

if (failures.length) {
  console.error('Public terminology check failed:');
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}

console.log('Public terminology check passed: no project-private coordinate labels in reader-facing prose.');
