// env.js — ad-block / tracking-protection probe (SoT-09). Uses only first-party
// honeypots (no real third-party ad/tracker requests), so it is privacy-safe
// and cannot itself harm the user. Results feed FP mitigation, never a lone
// bot verdict.

export async function probeEnvironment() {
  const env = {
    probed: true,
    adBlock: false,
    trackingProtection: false,
    doNotTrack: navigator.doNotTrack || '',
    gpc: !!navigator.globalPrivacyControl,
    collectorBlocked: false,
    braveShields: !!(navigator.brave),
  };

  // Ad-block bait: an element with ad-like classes is hidden by ad blockers.
  try {
    const bait = document.createElement('div');
    bait.className = 'adsbox ad-banner pub_300x250 banner_ad';
    bait.style.cssText = 'position:absolute;left:-9999px;height:10px;width:10px;';
    document.body.appendChild(bait);
    // give the blocker a tick to act
    await new Promise((r) => setTimeout(r, 60));
    env.adBlock = bait.offsetHeight === 0 || getComputedStyle(bait).display === 'none';
    bait.remove();
  } catch (_) {}

  // Tracking protection: fetch a first-party honeypot path that mimics a
  // tracker filename; if a content blocker matches the filter, the fetch fails.
  try {
    const ctrl = new AbortController();
    const to = setTimeout(() => ctrl.abort(), 400);
    await fetch('/static/honeypot/analytics.js', { signal: ctrl.signal, cache: 'no-store' });
    clearTimeout(to);
  } catch (_) {
    env.trackingProtection = true;
  }

  return env;
}
