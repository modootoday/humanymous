// fp_churn_proxy.mjs — mid-session fingerprintId rotation per exit hop to dodge
// classic proxy_rotation (fp|ja4 key) while hopping anonymous-proxy exits.
// Blue: l5.correlation.fp_churn_proxy → HR-19 DENY.
import { attack, hostOf } from './_bin.mjs';
export const label = 'bot:fp-churn-proxy';
export const needsBrowser = false;
export function run(baseURL) { return attack('fp-churn-proxy', hostOf(baseURL)); }
