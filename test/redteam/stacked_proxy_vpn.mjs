// stacked_proxy_vpn.mjs — Squid forward hop stacked on a VPN exit with WebRTC
// leak (proxy-over-VPN scraper stack). Blue: proxy_hop + vpn_webrtc_leak → HR-24.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:stacked-proxy-vpn';
export const needsBrowser = false;
export function run(baseURL) { return attack('stacked-proxy-vpn', hostOf(baseURL)); }
