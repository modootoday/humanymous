// gen.mjs — build-time generator for the humanymous docs site's discoverability assets.
// Produces, per documentation page:
//   1. a brand OG image (WebP)  → docs/assets/og/<path>.webp        (SEO / social cards)
//   2. an LLM-readable markdown → docs/assets/llms/<path>.llms.txt  (AEO / GEO)
// and a root docs/llms.txt index. Rendering uses the SoT-34 brand tokens + the
// pulse-aperture logo, so social/answer-engine cards match the product surface.
//
// Run: cd scripts/docsgen && npm install && node gen.mjs   (or `make docs-assets`).
// Not part of the shipped engine — the Go binaries and images never see this.

import sharp from 'sharp';
import { readdirSync, readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { join, dirname, relative, resolve } from 'node:path';

const ROOT = resolve(process.argv[2] || join(dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..', '..'));
const DOCS = join(ROOT, 'docs');
const SITE = 'https://humanymous.net';

// ── SoT-34 brand tokens (dark — the canonical OG surface) ──────────────────
const T = { bg:'#0a0e14', panel:'#111722', border:'#202a38', text:'#e4ecf5', muted:'#808fa3',
  faint:'#5a6879', accent:'#35d0ba', allow:'#3fb950', challenge:'#d9a441', deny:'#f0556a' };
const SANS = "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif";
const MONO = "ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace";

const SECTION = { 'start-here':'Start here','how-to':'How-to','tutorials':'Tutorial','reference':'Reference',
  'explanation':'Explanation','concepts':'Concept','runbooks':'Runbook','help':'Help' };

// Per-page SEO overrides produced by the docs-seo-panel workflow (url → {ogEyebrow,
// ogHeadline, description, keywords}). Absent → fall back to the page title + section.
const SEO = (() => { try { return JSON.parse(readFileSync(new URL('./seo.json', import.meta.url))); } catch { return {}; } })();

function esc(s){ return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }

// Word-wrap a title into lines that fit the card at the given font size.
function wrap(text, maxChars, maxLines){
  const words = text.split(/\s+/); const lines=[]; let cur='';
  for(const w of words){
    if((cur+' '+w).trim().length > maxChars){ if(cur) lines.push(cur); cur=w; }
    else cur=(cur+' '+w).trim();
    if(lines.length===maxLines-1 && (cur+' ').length>maxChars) break;
  }
  if(cur) lines.push(cur);
  if(lines.length>maxLines){ lines.length=maxLines; lines[maxLines-1]=lines[maxLines-1].replace(/.{1}$/,'…'); }
  return lines;
}

// The brand OG template (1200×630).
function ogSvg(title, section){
  const fs = title.length > 46 ? 58 : 68;
  const maxChars = title.length > 46 ? 30 : 26;
  const lines = wrap(title, maxChars, 3);
  // TOP-anchored: the first line sits at a fixed baseline well below the eyebrow and
  // the block grows DOWNWARD, so a 2- or 3-line title can never ride up into the
  // eyebrow (the previous centre-anchored math did, causing the overlap).
  const lineH = Math.round(fs * 1.18);
  const firstY = 300;
  const tspans = lines.map((l,i)=>`<tspan x="80" y="${firstY + i*lineH}">${esc(l)}</tspan>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <radialGradient id="glow" cx="82%" cy="6%" r="70%">
      <stop offset="0%" stop-color="${T.accent}" stop-opacity="0.14"/>
      <stop offset="55%" stop-color="${T.accent}" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="1200" height="630" fill="${T.bg}"/>
  <rect width="1200" height="630" fill="url(#glow)"/>
  <!-- hairline frame -->
  <rect x="0.5" y="0.5" width="1199" height="629" fill="none" stroke="${T.border}" stroke-width="1"/>
  <!-- pulse-aperture logo + wordmark -->
  <g transform="translate(80,78) scale(2.05)">
    <path d="M27.3 11.9 A12 12 0 1 1 20.1 4.7" stroke="${T.accent}" stroke-width="2" stroke-linecap="round" fill="none"/>
    <polyline points="6,17 11,17 14,17 16,10 18,22 21,17 26,17" fill="none" stroke="${T.accent}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
  </g>
  <text x="163" y="128" font-family="${SANS}" font-size="34" font-weight="600" letter-spacing="-0.5" fill="${T.text}">huma<tspan fill="${T.accent}">nym</tspan>ous</text>
  <!-- section eyebrow -->
  <text x="80" y="212" font-family="${MONO}" font-size="21" letter-spacing="3" fill="${T.faint}">${esc(section.toUpperCase())}</text>
  <!-- title -->
  <text font-family="${SANS}" font-size="${fs}" font-weight="650" letter-spacing="-1.5" fill="${T.text}">${tspans}</text>
  <!-- verdict-trio accent strip -->
  <g transform="translate(80,548)">
    <rect x="0" y="0" width="52" height="7" rx="3" fill="${T.allow}"/>
    <rect x="60" y="0" width="52" height="7" rx="3" fill="${T.challenge}"/>
    <rect x="120" y="0" width="52" height="7" rx="3" fill="${T.deny}"/>
  </g>
  <text x="80" y="536" font-family="${MONO}" font-size="22" fill="${T.muted}">Raise the cost of automation.</text>
  <text x="1120" y="588" text-anchor="end" font-family="${MONO}" font-size="18" fill="${T.faint}">humanymous.net</text>
</svg>`;
}

// ── walk docs/ for markdown pages (skip Jekyll internals + assets) ─────────
function walk(dir){
  let out=[];
  for(const e of readdirSync(dir)){
    if(e.startsWith('_') || e==='assets' || e==='.well-known' || e==='node_modules') continue;
    const p=join(dir,e);
    if(statSync(p).isDirectory()) out=out.concat(walk(p));
    else if(e.endsWith('.md') && e.toLowerCase() !== 'agents.md') out.push(p);
  }
  return out;
}

function parse(file){
  const raw = readFileSync(file,'utf8');
  let body = raw, fm = {};
  const m = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
  if(m){ body = raw.slice(m[0].length); for(const line of m[1].split(/\r?\n/)){ const kv=line.match(/^(\w+):\s*(.*)$/); if(kv) fm[kv[1]]=kv[2].replace(/^["']|["']$/g,''); } }
  const h1 = body.match(/^#\s+(.+)$/m);
  const title = fm.title || (h1 ? h1[1].trim() : 'humanymous');
  return { title, body, published: fm.published };
}

const pages = walk(DOCS);
let ogN=0, llmsN=0;
const index = [];

for(const file of pages){
  const rel = relative(DOCS, file).replace(/\\/g,'/');           // e.g. how-to/cut-a-release.md
  const isHome = rel.toLowerCase()==='readme.md';
  const urlPath = isHome ? '' : rel.replace(/\.md$/,'.html');
  const seg = rel.split('/');
  const section = isHome ? 'Overview' : (SECTION[seg[0]] || 'Documentation');
  const { title, body, published } = parse(file);
  if (published === 'false') continue; // maintainer-only pages are not published (no OG/llms/index entry)
  const seo = SEO[isHome ? '/' : urlPath] || {};

  // 1. OG WebP → docs/assets/og/<path>.webp   (home → default.webp). Use the panel's
  //    optimized OG headline/eyebrow when present, else the page title + section.
  const ogRel = isHome ? 'default' : rel.replace(/\.md$/,'');
  const ogOut = join(DOCS,'assets','og', ogRel + '.webp');
  mkdirSync(dirname(ogOut), { recursive:true });
  // Rasterize at 2× density (crisp text) then downsample to the 1200×630 OG spec.
  const webp = await sharp(Buffer.from(ogSvg(seo.ogHeadline || title, seo.ogEyebrow || section)), { density:144 })
    .resize(1200, 630).webp({ quality:80 }).toBuffer();
  writeFileSync(ogOut, webp); ogN++;

  // 2. LLM-readable markdown → docs/assets/llms/<path>.llms.txt
  const llmsRel = isHome ? 'index' : rel.replace(/\.md$/,'');
  const llmsOut = join(DOCS,'assets','llms', llmsRel + '.llms.txt');
  mkdirSync(dirname(llmsOut), { recursive:true });
  const canonical = SITE + '/' + urlPath;
  writeFileSync(llmsOut, `# ${title}\n\nSource: ${canonical}\nProject: humanymous — defensive anti-bot / browser-automation detection (Apache-2.0).\n\n---\n\n${body.trim()}\n`);
  llmsN++;

  if(!isHome) index.push({ section, title, url: SITE + '/' + urlPath });
}

// 3. Root llms.txt index (llmstxt.org convention)
const bySection = {};
for(const p of index){ (bySection[p.section] ||= []).push(p); }
const order = ['Start here','Tutorial','How-to','Reference','Explanation','Concept','Runbook','Help','Documentation'];
let llms = `# humanymous\n\n> humanymous is an Apache-2.0 reference implementation for reducing browser automation against an application you operate. The standalone Core engine combines browser, integrity, interaction, connection, and consistency evidence into an ALLOW, CHALLENGE, or DENY verdict. The Gate reverse proxy enforces its own request-time verdict at the edge and records decisions in a tamper-evident local audit log. Core and Gate share scoring code but do not observe identical evidence.\n\nThe project raises the cost of commodity automation; it does not prove that a caller is human or promise to stop coherent automation using a real browser and plausible network origin. Gate is an application-edge automation control, not a web application firewall, content delivery network, traffic-absorption service, or interactive human-verification product. The computational-work challenge is incomplete as a production recovery and accessibility path.\n\n`;
for(const s of order){
  if(!bySection[s]) continue;
  llms += `## ${s}\n\n`;
  for(const p of bySection[s]) llms += `- [${p.title}](${p.url})\n`;
  llms += `\n`;
}
llms += `## Source\n\n- [GitHub repository](https://github.com/modootoday/humanymous)\n- Per-page LLM-readable markdown lives under \`/assets/llms/<path>.llms.txt\` and is linked from each page's \`<link rel="alternate" type="text/markdown">\`.\n`;
writeFileSync(join(DOCS,'llms.txt'), llms);

console.log(`docsgen: ${ogN} OG WebP + ${llmsN} llms.txt pages + root llms.txt (${index.length} indexed) → docs/assets/{og,llms}/`);
