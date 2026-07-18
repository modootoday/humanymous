// video_scrape.mjs — media bandwidth-abuse attack (SoT-10): fire many parallel
// Range requests at a heavy video resource (a download-manager / scraper
// pattern), then read the verdict. The Blue engine flags l5.media.range_storm
// and the gate denies the full video (traffic saved).

export const label = 'bot:video-scrape';
export const needsBrowser = false;

export async function run(baseURL) {
  const base = baseURL.replace(/\/$/, '');
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

  // Establish a session (cookie).
  const s = await fetch(base + '/api/session');
  const cookie = (s.headers.get('set-cookie') || '').split(';')[0];
  const H = { Cookie: cookie, 'User-Agent': 'Mozilla/5.0 Chrome/126' };

  // Parallel Range storm on the heavy resource.
  await Promise.all(
    Array.from({ length: 12 }, (_, i) =>
      fetch(base + '/res/media/sample.mp4', {
        headers: { ...H, Range: `bytes=${i * 100000}-${i * 100000 + 99999}` },
      }).catch(() => {})
    )
  );

  // Report to /api/collect to get the scored verdict (media signals accrued).
  const res = await fetch(base + '/api/collect?label=' + encodeURIComponent(label), {
    method: 'POST',
    headers: { ...H, 'Content-Type': 'application/json' },
    body: JSON.stringify({ userAgent: 'Mozilla/5.0 Chrome/126', signals: [] }),
  });
  return res.json();
}
