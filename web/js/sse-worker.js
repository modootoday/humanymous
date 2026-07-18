// sse-worker.js — optional off-main-thread SSE ingest (SoT-33). Below a few hundred
// events/sec the main-thread ring + rAF in feed.js is enough; past that, run this
// Worker so JSON parsing never steals from the paint budget. It owns the
// EventSource, parses, and posts ONE batch per ~frame to the main thread, which
// feeds them to createFeed().pushMany() — the render path is unchanged.
//
// Usage (main thread):
//   const w = new Worker('/static/js/sse-worker.js');
//   w.postMessage({ url:'/playground/events', events:['session.scored','attack.ran'] });
//   w.onmessage = e => feed.pushMany(e.data);   // e.data = [{type,data}, ...]
//
// Workers cannot touch the DOM; this only parses + batches. EventSource is available
// in Workers in modern browsers; if it is not, the main-thread path in feed.js still
// works. For a shared-memory ring (zero-copy) enable cross-origin isolation
// (COOP:same-origin + COEP:require-corp) first and verify it does not break gated
// embeds — ship this postMessage path first (progressive enhancement).

let es = null;
let batch = [];
let timer = null;

function flush() {
  timer = null;
  if (batch.length) { postMessage(batch); batch = []; }
}
function enqueue(rec) {
  batch.push(rec);
  if (!timer) timer = setTimeout(flush, 16); // coalesce ~one frame of events per post
}

onmessage = (e) => {
  const msg = e.data || {};
  if (!msg.url) return;
  if (es) es.close();
  es = new EventSource(msg.url);
  (msg.events || ['message']).forEach((t) => {
    es.addEventListener(t, (ev) => {
      let d = null;
      try { d = JSON.parse(ev.data); } catch (_) { d = { raw: ev.data }; }
      enqueue({ type: t, data: d });
    });
  });
};
