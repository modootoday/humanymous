// frame.mjs — wrap a captured surface screenshot in a brand-consistent macOS window
// chrome and emit a theme-aware WebP. Pure sharp (the same dependency gen.mjs uses):
// the frame is composited from SVG chrome + the raster capture, so it is offline and
// deterministic. The captures themselves are produced by shoot.mjs (headless render of
// the real console/demo/pass HTML — no live server, no stale screenshots).
//
// This module is a library: shoot.mjs imports frameContent() and passes the Playwright
// screenshot buffer directly. The window *content* is the product's own dark UI; only
// the window chrome + shadow adapt to the requested theme, so a <picture> can show each
// reader the chrome that matches their light/dark preference.

import sharp from 'sharp';

// Window-chrome geometry (output px; captures are hi-dpi, embedded at ~half width).
const TB = 52;      // titlebar height
const R = 22;       // corner radius
const PAD = 64;     // shadow padding around the window
const DOT = 13;     // traffic-light diameter
const GAP = 9;      // gap between traffic lights
const DOTX = 23;    // first traffic light centre-x
const MAXW = 1800;  // hard cap on the emitted width

const LIGHTS = ['#ff5f57', '#febc2e', '#28c840']; // macOS traffic lights (fixed both themes)

// Per-theme window chrome. The screenshot inside is unchanged; only the frame adapts.
export const THEMES = {
  dark:  { bar1: '#1c2430', bar2: '#151c26', border: '#253040', title: '#8a99ad', panel: '#0b1119', shHex: '#04070c', shOp: 0.55 },
  light: { bar1: '#f7f8fa', bar2: '#eceef2', border: '#d6dbe3', title: '#5a6572', panel: '#ffffff', shHex: '#0a101a', shOp: 0.22 },
};

const MONO = "ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace";
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const svg = (s) => Buffer.from(s);

function titlebarSvg(w, th, title) {
  const cy = TB / 2, baseY = Math.round(TB / 2) + 6;
  const dots = LIGHTS.map((c, i) =>
    `<circle cx="${DOTX + i * (DOT + GAP)}" cy="${cy}" r="${DOT / 2}" fill="${c}" fill-opacity="0.95"/>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${TB}">
    <defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="${th.bar1}"/><stop offset="1" stop-color="${th.bar2}"/>
    </linearGradient></defs>
    <rect width="${w}" height="${TB}" fill="url(#g)"/>
    <line x1="0" y1="${TB - 0.5}" x2="${w}" y2="${TB - 0.5}" stroke="${th.border}" stroke-width="1"/>
    ${dots}
    <text x="${w / 2}" y="${baseY}" text-anchor="middle" font-family="${MONO}" font-size="17" fill="${th.title}">${esc(title)}</text>
  </svg>`;
}

const maskSvg = (w, h) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><rect width="${w}" height="${h}" rx="${R}" ry="${R}" fill="#fff"/></svg>`;

const borderSvg = (w, h, th) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><rect x="0.75" y="0.75" width="${w - 1.5}" height="${h - 1.5}" rx="${R}" ry="${R}" fill="none" stroke="${th.border}" stroke-width="1.5"/></svg>`;

const shadowSvg = (fw, fh, winW, winH, th) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${fw}" height="${fh}">
    <defs><filter id="b" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="30"/></filter></defs>
    <rect x="${PAD}" y="${PAD + 12}" width="${winW}" height="${winH}" rx="${R}" ry="${R}" fill="${th.shHex}" fill-opacity="${th.shOp}" filter="url(#b)"/>
  </svg>`;

// frameContent(pngBuffer, {title, w, theme, trim}) → WebP buffer of the framed window.
// trim (default true) removes uniform empty margins; pass trim:false when the capture
// deliberately includes the app's own background as internal padding (else the uniform
// side columns get trimmed away).
export async function frameContent(pngBuffer, { title, w, theme, trim = true }) {
  const th = THEMES[theme];
  // optionally trim uniform empty margins, then size the capture to the content width.
  let pipe = sharp(pngBuffer);
  if (trim) pipe = pipe.trim({ threshold: 14 });
  const content = await pipe.resize({ width: w }).toBuffer();
  const cm = await sharp(content).metadata();
  const winW = w, winH = TB + cm.height;

  // panel bg + capture + titlebar → clip corners → crisp outline on top.
  let win = await sharp({ create: { width: winW, height: winH, channels: 4, background: th.panel } })
    .composite([
      { input: content, top: TB, left: 0 },
      { input: svg(titlebarSvg(winW, th, title)), top: 0, left: 0 },
    ]).png().toBuffer();
  win = await sharp(win).composite([{ input: svg(maskSvg(winW, winH)), blend: 'dest-in' }]).png().toBuffer();
  win = await sharp(win).composite([{ input: svg(borderSvg(winW, winH, th)), top: 0, left: 0 }]).png().toBuffer();

  // soft shadow behind the window, on a transparent canvas so it drops onto any page bg.
  const fw = winW + 2 * PAD, fh = winH + 2 * PAD;
  let out = sharp({ create: { width: fw, height: fh, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite([
      { input: svg(shadowSvg(fw, fh, winW, winH, th)) },
      { input: win, top: PAD, left: PAD },
    ]);
  if (fw > MAXW) out = out.resize({ width: MAXW });
  return out.webp({ quality: 82, effort: 5 }).toBuffer();
}
