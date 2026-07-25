// cdn_ip_spoof.mjs — forges CF-Connecting-IP / True-Client-IP / X-Client-IP to
// launder identity through an anonymous proxy. Browsers never send these to origin.
// Blue: l5.header.client_ip_spoof → HR-24.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:cdn-ip-spoof';
export const needsBrowser = false;
export function run(baseURL) { return attack('cdn-ip-spoof', hostOf(baseURL)); }
