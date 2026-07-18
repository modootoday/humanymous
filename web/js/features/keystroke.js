// features/keystroke.js — reduce keydown/keyup timing pairs to KeyFeatures
// (SoT-03 §2). Only timings are used; key values are never recorded (privacy).

export function keyFeatures(downs, ups, pasteCount) {
  // downs/ups: [{code,t}] in arrival order. Pair by code for dwell; use press
  // sequence for flight.
  const dwellList = [];
  const pending = new Map();
  let zeroDwell = 0;
  for (const d of downs) pending.set(d.code, d.t);
  for (const u of ups) {
    const dt = pending.get(u.code);
    if (dt !== undefined) {
      const dwell = u.t - dt;
      dwellList.push(dwell);
      if (dwell <= 1) zeroDwell++;
      pending.delete(u.code);
    }
  }
  const flightList = [];
  for (let i = 1; i < downs.length; i++) flightList.push(downs[i].t - downs[i - 1].t);

  const ks = downs.length;
  return {
    keystrokes: ks,
    meanDwellMs: mean(dwellList),
    dwellStdDevMs: stddev(dwellList),
    meanFlightMs: mean(flightList),
    flightStdDevMs: stddev(flightList),
    pasteEvents: pasteCount,
    zeroDwellFrac: ks ? zeroDwell / ks : 0,
  };
}

function mean(a) { return a.length ? a.reduce((s, x) => s + x, 0) / a.length : 0; }
function stddev(a) {
  if (a.length < 2) return 0;
  const m = mean(a);
  return Math.sqrt(a.reduce((s, x) => s + (x - m) * (x - m), 0) / a.length);
}
