// watermark_strip.mjs — resource-leak + anti-forensics attack (SoT-08): download
// a watermarked image, strip ALL metadata by re-encoding the pixels, then check
// whether the Blue engine can still trace the leak. The pixel-LSB channel
// survives the strip, so /api/trace still identifies the leaking session — the
// "attack" fails and the leak is attributed (reported as a Blue win).

export const label = 'bot:watermark-strip';
export const needsBrowser = false;

export async function run(baseURL) {
  const base = baseURL.replace(/\/$/, '');
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

  const s = await fetch(base + '/api/session');
  const cookie = (s.headers.get('set-cookie') || '').split(';')[0];

  // Download the per-session watermarked PNG.
  const imgResp = await fetch(base + '/res/img/sample.png', { headers: { Cookie: cookie } });
  const watermarked = Buffer.from(await imgResp.arrayBuffer());

  // Attacker strips metadata (tEXt) but cannot easily scrub the pixel LSBs.
  const stripped = stripPngAncillary(watermarked);

  // Blue traces the leaked (stripped) image.
  const trace = await fetch(base + '/api/trace', {
    method: 'POST',
    headers: { 'Content-Type': 'image/png' },
    body: stripped,
  });
  const t = await trace.json();

  // A successful trace means the leaker is identified despite the strip.
  const traced = !!t.traced && !t.result?.forged;
  return {
    verdict: traced ? 'DENY' : 'ALLOW',
    hardRuleFired: traced ? 'wm-traced' : '',
    riskScore: traced ? 100 : 0,
    topContributors: [{ id: traced ? 'l5.resource.watermark_trace(' + (t.result?.channel || '?') + ')' : 'wm-lost', score: 100 }],
    sessionId: t.result?.sessionId,
  };
}

// stripPngAncillary removes all ancillary chunks (incl. tEXt), keeping only the
// critical chunks (IHDR/PLTE/IDAT/IEND) — the metadata-strip attack.
function stripPngAncillary(png) {
  const sig = png.subarray(0, 8);
  const out = [sig];
  let pos = 8;
  const critical = new Set(['IHDR', 'PLTE', 'IDAT', 'IEND']);
  while (pos + 8 <= png.length) {
    const len = png.readUInt32BE(pos);
    const type = png.subarray(pos + 4, pos + 8).toString('latin1');
    const end = pos + 12 + len;
    if (critical.has(type)) out.push(png.subarray(pos, end));
    pos = end;
  }
  return Buffer.concat(out);
}
