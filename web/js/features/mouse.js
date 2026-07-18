// features/mouse.js — reduce a buffer of pointer samples to MouseFeatures
// (SoT-03 §1). Input: [{x,y,t}]. Output matches Go signals.MouseFeatures.

export function mouseFeatures(samples) {
  const n = samples.length;
  const f = {
    samples: n, meanCurvature: 0, velocityStdDev: 0, accelEntropy: 0,
    straightLineFrac: 0, pauseCount: 0, maxTeleportPx: 0, meanJerk: 0, coalescedRatio: 0,
  };
  if (n < 3) return f;

  const vels = [], accels = [];
  let pathLen = 0, straightSegs = 0, segs = 0, maxJump = 0;
  for (let i = 1; i < n; i++) {
    const dx = samples[i].x - samples[i - 1].x;
    const dy = samples[i].y - samples[i - 1].y;
    const dt = Math.max(1, samples[i].t - samples[i - 1].t);
    const d = Math.hypot(dx, dy);
    pathLen += d;
    if (d > maxJump) maxJump = d;
    vels.push(d / dt);
    if (samples[i].t - samples[i - 1].t > 120) f.pauseCount++;
  }
  for (let i = 1; i < vels.length; i++) accels.push(vels[i] - vels[i - 1]);

  // straightness over sliding windows of 3 points.
  for (let i = 2; i < n; i++) {
    const chord = Math.hypot(samples[i].x - samples[i - 2].x, samples[i].y - samples[i - 2].y);
    const arc = Math.hypot(samples[i].x - samples[i - 1].x, samples[i].y - samples[i - 1].y) +
      Math.hypot(samples[i - 1].x - samples[i - 2].x, samples[i - 1].y - samples[i - 2].y);
    if (arc > 0) {
      segs++;
      if (chord / arc > 0.995) straightSegs++;
    }
  }

  // jerk = Δacceleration (higher-order smoothness). Human motion has bounded,
  // continuous jerk; teleport/scripted motion spikes or flatlines (SoT-14).
  const jerks = [];
  for (let i = 1; i < accels.length; i++) jerks.push(Math.abs(accels[i] - accels[i - 1]));
  f.meanJerk = jerks.length ? jerks.reduce((s, x) => s + x, 0) / jerks.length : 0;

  f.velocityStdDev = stddev(vels);
  f.accelEntropy = entropy(accels);
  f.straightLineFrac = segs ? straightSegs / segs : 0;
  f.maxTeleportPx = maxJump;
  // mean curvature approximated as 1 - (straight fraction) scaled small.
  f.meanCurvature = segs ? (1 - f.straightLineFrac) * 0.01 : 0;
  return f;
}

function stddev(a) {
  if (a.length < 2) return 0;
  const m = a.reduce((s, x) => s + x, 0) / a.length;
  const v = a.reduce((s, x) => s + (x - m) * (x - m), 0) / a.length;
  return Math.sqrt(v);
}

// Shannon entropy over 12 acceleration bins (normalized to 0..1-ish).
function entropy(a) {
  if (a.length < 2) return 0;
  const bins = new Array(12).fill(0);
  const max = Math.max(...a.map(Math.abs)) || 1;
  for (const x of a) {
    const idx = Math.min(11, Math.floor((Math.abs(x) / max) * 11));
    bins[idx]++;
  }
  let h = 0;
  for (const c of bins) {
    if (!c) continue;
    const p = c / a.length;
    h -= p * Math.log2(p);
  }
  return h;
}
