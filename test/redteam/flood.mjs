// flood.mjs — application-layer request flood / credential-stuffing velocity from
// one TLS fingerprint (SoT-17). The Blue engine meters by JA4+subnet (not IP), so
// the flood is caught even across rotated sessions -> l5.abuse.flood -> HR-21 DENY.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:flood';
export const needsBrowser = false;
export function run(baseURL) { return attack('flood', hostOf(baseURL)); }
