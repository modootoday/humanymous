import type { IncomingMessage, ServerResponse } from 'node:http';
export interface Envelope {
  protocol: 'humanymous.adapter/0.1'; id: string; tenant: string; origin: string; revision: string;
  deadline: number; method: string; target: string; authority: string; peer: string | null;
  headers: [string,string][]; headerMode: 'node-raw' | 'fetch-normalized';
  body: { layer: 'transfer-decoded-content-encoded'; complete: true; length: number; sha256: string; base64: string };
  capabilities: { rawTls: false; originalH2: false; rawHeaderOrder: boolean; completeBody: true }; stream: null;
  adapter: {id:'humanymous.node';version:'0.1.0-experimental.1';installation:string}; connectionId:string|null; capture:null;
}
export interface Decision { protocol: string; id: string; tenant: string; origin: string; revision: string; expires: number; verdict: 'ALLOW'|'DENY'|'CHALLENGE' }
export interface Options { installation:string; tenant: string; origin: string; revision: string; routes: string[]; reserved?: string; maxBody?: number; timeoutMs?: number; socketPath?: string; credential?: string; decide?: (request: Envelope, signal: AbortSignal) => Promise<Decision> }
export const VERSION: 'humanymous.adapter/0.1';
export function localClient(options: {socketPath:string;credential:string;timeoutMs?:number}): (request:Envelope,signal:AbortSignal)=>Promise<Decision>;
export function express(options:Options): (req:IncomingMessage,res:ServerResponse,next:()=>void)=>Promise<void>;
export function fetchAdapter<C>(options:Options,handler:(request:Request,context:C)=>Response|Promise<Response>): (request:Request,context:C)=>Promise<Response>;
export function hono<E>(options:Options,app:{routes:{path:string}[];fetch:(request:Request,env?:E)=>Response|Promise<Response>}): (request:Request,env?:E)=>Promise<Response>;
export function htmlHook(options:{nonce:string;scriptPath?:string}):{markup:string;cspSource:string};
