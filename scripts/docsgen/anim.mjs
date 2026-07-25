// anim.mjs — render the animated docs sources (scripts/docsgen/mocks/*.html) frame by
// frame headlessly and encode each to the two forms the targets need:
//   • animated WebP  → docs/assets/screenshots/anim/<name>.webp   (github.com <img>, -loop 0)
//   • WebM (VP9) + MP4 (H.264) + poster WebP → the docs-site <video autoplay loop muted>
// The mocks expose window.__frame(i,N) so frames are deterministic and the loop is seamless.
// Uses the same playwright-core browser as shoot.mjs, sharp for the animated WebP, and the
// system ffmpeg (on PATH) for the video encodes.
//
// Run: cd scripts/docsgen && node anim.mjs   (or `make docs-anim`).

import { writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import sharp from 'sharp';

const HERE = dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
const ROOT = resolve(join(HERE, '..', '..'));
const MOCKS = join(HERE, 'mocks');
const OUT = join(ROOT, 'docs', 'assets', 'screenshots', 'anim');
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright-core', { paths: [join(ROOT, 'test', 'node_modules')] }));

const ANIMS = [
  { name: 'ledger-live', file: 'ledger-live.html', frames: 105, fps: 15, webpMax: 820, poster: 0.5 },
  { name: 'quickstart-cast', file: 'terminal-cast.html', frames: 150, fps: 13, webpMax: 820, poster: 0.9 },
];

const pad = (i) => String(i).padStart(4, '0');
const kb = (b) => Math.round(b.length / 1024);

async function launch() {
  for (const c of [{ channel: 'msedge' }, { channel: undefined }]) {
    try { return await chromium.launch({ ...c, headless: true }); } catch { /* next */ }
  }
  throw new Error('no browser (Edge or bundled Chromium)');
}

async function run() {
  mkdirSync(OUT, { recursive: true });
  const browser = await launch();
  for (const a of ANIMS) {
    const tmp = join(tmpdir(), 'hmn-anim-' + a.name);
    rmSync(tmp, { recursive: true, force: true }); mkdirSync(tmp, { recursive: true });

    // 1. capture N deterministic frames
    const ctx = await browser.newContext({ viewport: { width: 916, height: 600 }, deviceScaleFactor: 2, locale: 'en-US' });
    const page = await ctx.newPage();
    await page.goto(pathToFileURL(join(MOCKS, a.file)).href, { waitUntil: 'load' });
    await page.waitForFunction('typeof window.__frame === "function"');
    const stage = page.locator('.stage');
    for (let i = 0; i < a.frames; i++) {
      await page.evaluate(([f, n]) => window.__frame(f, n), [i, a.frames]);
      writeFileSync(join(tmp, 'f-' + pad(i) + '.png'), await stage.screenshot({ type: 'png' }));
    }
    await ctx.close();

    // 2. animated WebP for github.com (resize to a sane width, join as animation pages)
    const resized = [];
    for (let i = 0; i < a.frames; i++) resized.push(await sharp(join(tmp, 'f-' + pad(i) + '.png')).resize({ width: a.webpMax }).png().toBuffer());
    const webp = await sharp(resized, { join: { across: 1, animated: true } })
      .webp({ loop: 0, delay: Math.round(1000 / a.fps), quality: 68, effort: 4 }).toBuffer();
    writeFileSync(join(OUT, a.name + '.webp'), webp);

    // 3. WebM (VP9) + MP4 (H.264) for the docs <video>; even dims for yuv420p
    const inpat = join(tmp, 'f-%04d.png');
    const evenScale = 'scale=trunc(iw/2)*2:trunc(ih/2)*2';
    execFileSync('ffmpeg', ['-y', '-hide_banner', '-loglevel', 'error', '-framerate', String(a.fps), '-i', inpat,
      '-c:v', 'libvpx-vp9', '-pix_fmt', 'yuv420p', '-b:v', '0', '-crf', '30', '-an', '-vf', evenScale, join(OUT, a.name + '.webm')]);
    execFileSync('ffmpeg', ['-y', '-hide_banner', '-loglevel', 'error', '-framerate', String(a.fps), '-i', inpat,
      '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '23', '-an', '-movflags', '+faststart', '-vf', evenScale, join(OUT, a.name + '.mp4')]);

    // 4. poster still for <video poster>
    const posterBuf = await sharp(join(tmp, 'f-' + pad(Math.round(a.poster * (a.frames - 1))) + '.png')).resize({ width: a.webpMax }).webp({ quality: 82 }).toBuffer();
    writeFileSync(join(OUT, a.name + '-poster.webp'), posterBuf);

    rmSync(tmp, { recursive: true, force: true });
    console.log(`  ✓ ${a.name}  ${a.frames}f @${a.fps}fps · webp ${kb(webp)}KB`);
  }
  await browser.close();
  console.log(`anim: ${ANIMS.length} animations → docs/assets/screenshots/anim/ (webp + webm + mp4 + poster)`);
}

run().catch((e) => { console.error(e); process.exit(1); });
