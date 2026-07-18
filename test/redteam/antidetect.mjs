// antidetect.mjs — anti-detect browser signature (Multilogin/GoLogin/AdsPower,
// SoT-04): a consistent, internally-coherent spoofed fingerprint plus per-profile
// proxy, driving with no human behavior. Modeled as headful Edge with a coherent
// spoofed canvas/webgl and NO interaction => HR-12 (behavior). In production the
// datacenter/proxy IP and entropy anomalies add to this.
import { drive } from './_driver.mjs';
export const label = 'bot:anti-detect-browser';
export const needsBrowser = true;
const coherentSpoof = () => {
  // A coherent (not contradictory) fingerprint: consistent GPU strings.
  const g = WebGLRenderingContext.prototype.getParameter;
  WebGLRenderingContext.prototype.getParameter = function (p) {
    if (p === 37445) return 'Google Inc. (NVIDIA)';
    if (p === 37446) return 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Direct3D11 vs_5_0 ps_5_0, D3D11)';
    return g.call(this, p);
  };
};
export async function run(baseURL) {
  return drive(baseURL, {
    headless: false,
    launchArgs: ['--disable-blink-features=AutomationControlled'],
    initScripts: [coherentSpoof],
  });
}
