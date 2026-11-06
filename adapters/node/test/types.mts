import { express, fetchAdapter, htmlHook, VERSION } from '@humanymous/node';
const options = {installation:'test',tenant:'t',origin:'o',revision:'r',routes:['/'],socketPath:'/run/decision.sock',credential:'a'.repeat(32)};
express(options);
fetchAdapter(options, () => new Response(VERSION));
htmlHook({nonce:'a'.repeat(24)});
