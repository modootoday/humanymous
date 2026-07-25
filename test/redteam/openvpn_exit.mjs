// openvpn_exit.mjs — OpenVPN L3 tunnel residual: TCP arrives from the VPN exit
// while WebRTC STUN still reveals a different public IP (classic VPN leak).
// Blue: l5.proxy.vpn_webrtc_leak → HR-24 CHALLENGE.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:openvpn-exit';
export const needsBrowser = false;
export function run(baseURL) { return attack('openvpn-exit', hostOf(baseURL)); }
