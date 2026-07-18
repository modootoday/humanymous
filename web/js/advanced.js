// advanced.js — collects advanced fingerprint surfaces (SoT-14): WebRTC media
// devices, speech voices, Widevine EME, WebGPU, AudioContext sample rate,
// Network Information rtt, battery, timezone/locale, matchMedia display profile,
// and Chrome memory-API presence. Absences are weak tells; the power is in
// cross-surface consistency, scored server-side. All probes are best-effort and
// never throw out.

// computeFingerprintId hashes the browser's stable, hard-to-change surfaces
// (canvas, WebGL, screen, timezone, hardware) into a per-device id used for
// cross-session correlation (SoT-15). It stays constant across sessions of the
// same browser, so a bot behind a rotating residential-proxy pool keeps ONE id
// while its IP changes — the pattern IP reputation cannot catch.
export async function computeFingerprintId() {
  const parts = [];
  try {
    const c = document.createElement('canvas');
    c.width = 240; c.height = 60;
    const ctx = c.getContext('2d');
    ctx.textBaseline = 'top'; ctx.font = '14px Arial';
    ctx.fillStyle = '#f60'; ctx.fillRect(2, 2, 100, 20);
    ctx.fillStyle = '#069'; ctx.fillText('humanymous-fp-\u{1F512}', 4, 4);
    parts.push(c.toDataURL());
  } catch (_) {}
  try {
    const gl = document.createElement('canvas').getContext('webgl');
    const d = gl && gl.getExtension('WEBGL_debug_renderer_info');
    if (d) parts.push(gl.getParameter(d.UNMASKED_VENDOR_WEBGL) + '/' + gl.getParameter(d.UNMASKED_RENDERER_WEBGL));
  } catch (_) {}
  parts.push([screen.width, screen.height, screen.colorDepth, devicePixelRatio].join('x'));
  try { parts.push(Intl.DateTimeFormat().resolvedOptions().timeZone); } catch (_) {}
  parts.push((navigator.languages || []).join(','));
  parts.push(String(navigator.hardwareConcurrency || 0));
  parts.push(navigator.platform || '');
  try {
    const enc = new TextEncoder();
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', enc.encode(parts.join('|'))));
    return [...digest.slice(0, 12)].map((b) => b.toString(16).padStart(2, '0')).join('');
  } catch (_) {
    // fallback non-crypto hash
    let h = 0; const s = parts.join('|');
    for (let i = 0; i < s.length; i++) { h = (h * 31 + s.charCodeAt(i)) | 0; }
    return (h >>> 0).toString(16);
  }
}

// withTimeout guards a promise so a probe that hangs (e.g. Widevine/WebGPU in
// some Firefox builds) can never stall the whole collection.
function withTimeout(promise, ms, fallback) {
  return Promise.race([
    Promise.resolve(promise).catch(() => fallback),
    new Promise((r) => setTimeout(() => r(fallback), ms)),
  ]);
}

export async function collectAdvanced() {
  const a = {
    probed: true,
    mediaDeviceCount: 0, hasAudioInput: false, hasVideoInput: false,
    voiceCount: 0, widevineSupported: false,
    webgpuPresent: false, webgpuVendor: '', webgpuFallback: false, webglVendor: '',
    audioSampleRate: 0,
    connectionPresent: false, connectionRtt: -1,
    batteryPresent: false, batteryLevel: -1, batteryCharging: false,
    timezoneIana: '', timezoneOffsetMin: 0, language: navigator.language || '',
    colorGamut: '', dynamicRange: '', pointerCoarse: false,
    maxTouchPoints: navigator.maxTouchPoints || 0,
    perfMemoryPresent: !!(performance && performance.memory),
    measureMemThrows: false,
    mobileUA: /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent),
  };

  // WebRTC media devices.
  try {
    const devs = await navigator.mediaDevices.enumerateDevices();
    a.mediaDeviceCount = devs.length;
    a.hasAudioInput = devs.some((d) => d.kind === 'audioinput');
    a.hasVideoInput = devs.some((d) => d.kind === 'videoinput');
  } catch (_) {}

  // Speech synthesis voices (async populate).
  try {
    let voices = speechSynthesis.getVoices();
    if (!voices.length) {
      await new Promise((r) => { speechSynthesis.onvoiceschanged = r; setTimeout(r, 300); });
      voices = speechSynthesis.getVoices();
    }
    a.voiceCount = voices.length;
  } catch (_) {}

  // Widevine EME — plain Chromium (Playwright default) rejects. Timeout-guarded
  // (can hang in some Firefox builds).
  a.widevineSupported = await withTimeout((async () => {
    await navigator.requestMediaKeySystemAccess('com.widevine.alpha', [{
      initDataTypes: ['cenc'],
      audioCapabilities: [{ contentType: 'audio/mp4;codecs="mp4a.40.2"' }],
      videoCapabilities: [{ contentType: 'video/mp4;codecs="avc1.42E01E"' }],
    }]);
    return true;
  })(), 800, false);

  // WebGL vendor.
  try {
    const gl = document.createElement('canvas').getContext('webgl');
    const dbg = gl && gl.getExtension('WEBGL_debug_renderer_info');
    if (dbg) a.webglVendor = String(gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL) || '');
  } catch (_) {}

  // WebGPU adapter — null/fallback under headless; vendor cross-check vs WebGL.
  try {
    if (navigator.gpu) {
      const adapter = await withTimeout(navigator.gpu.requestAdapter(), 800, null);
      if (adapter) {
        a.webgpuPresent = true;
        a.webgpuFallback = !!adapter.isFallbackAdapter;
        const info = adapter.info || (adapter.requestAdapterInfo ? await adapter.requestAdapterInfo() : null);
        if (info) a.webgpuVendor = String(info.vendor || '');
      }
    }
  } catch (_) {}

  // AudioContext sample rate — headless reports 24000.
  try {
    const Ctx = window.AudioContext || window.webkitAudioContext;
    const ctx = new Ctx();
    a.audioSampleRate = ctx.sampleRate;
    ctx.close();
  } catch (_) {}

  // Network Information rtt — headless/datacenter reports 0.
  try {
    if (navigator.connection) {
      a.connectionPresent = true;
      a.connectionRtt = navigator.connection.rtt;
    }
  } catch (_) {}

  // Battery — permanent level:1/charging:true on servers.
  try {
    if (navigator.getBattery) {
      const bat = await navigator.getBattery();
      a.batteryPresent = true;
      a.batteryLevel = bat.level;
      a.batteryCharging = bat.charging;
    }
  } catch (_) {}

  // Timezone / locale.
  try {
    a.timezoneIana = Intl.DateTimeFormat().resolvedOptions().timeZone || '';
    a.timezoneOffsetMin = new Date().getTimezoneOffset();
  } catch (_) {}

  // matchMedia display profile.
  try {
    a.colorGamut = matchMedia('(color-gamut: p3)').matches ? 'p3'
      : matchMedia('(color-gamut: srgb)').matches ? 'srgb' : '';
    a.dynamicRange = matchMedia('(dynamic-range: high)').matches ? 'high' : 'standard';
    a.pointerCoarse = matchMedia('(pointer: coarse)').matches;
  } catch (_) {}

  // measureUserAgentSpecificMemory throws in new-headless.
  try {
    if (performance.measureUserAgentSpecificMemory) {
      await performance.measureUserAgentSpecificMemory();
    }
  } catch (_) { a.measureMemThrows = true; }

  return a;
}
