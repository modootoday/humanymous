// feed.js — framework-free high-velocity log-feed renderer (SoT-33). The reusable
// "flexible, maintainable, ultra-fast" streaming architecture: it decouples ingest
// rate from paint rate so a feed arriving at thousands of events/sec never janks.
//
//   push(item)  -> O(1), buffers into a fixed-capacity ring (no DOM touch)
//   a single requestAnimationFrame loop is the ONLY DOM writer and coalesces every
//   event buffered since the last frame into ONE update.
//
// Bounded ring + rendered-row cap + a dropped-count callback keep the view honest
// (lossy under extreme load, never blocked); pause() freezes rendering while the
// ring stays live, so resume() shows current state, not a stale backlog.
//
// For >~thousands/sec, feed the ring from a Web Worker (see sse-worker.js) so JSON
// parsing runs off the main thread — the render path here is unchanged.

export function createFeed(opts) {
  const mount = opts.mount;
  const render = opts.render;
  const ringCap = opts.ringCap || 5000;
  const maxRows = opts.maxRows || 200;
  const onOverflow = opts.onOverflow;

  let ring = [];
  let dropped = 0;
  let scheduled = false;
  let paused = false;

  function schedule() {
    if (!scheduled && !paused) { scheduled = true; requestAnimationFrame(flush); }
  }
  function flush() {
    scheduled = false;
    if (paused || ring.length === 0) return;
    const batch = ring; ring = [];
    render(batch, { mount, maxRows }); // ONE DOM write for the whole frame's batch
  }

  return {
    push(item) {
      if (ring.length >= ringCap) { ring.shift(); dropped++; if (onOverflow) onOverflow(dropped); }
      ring.push(item);
      schedule();
    },
    pushMany(items) {
      for (let i = 0; i < items.length; i++) {
        if (ring.length >= ringCap) { ring.shift(); dropped++; if (onOverflow) onOverflow(dropped); }
        ring.push(items[i]);
      }
      schedule();
    },
    pause() { paused = true; },
    resume() { paused = false; schedule(); },
    get dropped() { return dropped; },
    get buffered() { return ring.length; },
  };
}

// prependRows renders a batch newest-first into `mount` in a single DOM write and
// caps the rendered node count (constant node count regardless of stream size).
// `toEl(item)` builds one row element. Pair with CSS `content-visibility:auto` on
// the rows so off-screen rows skip layout/paint.
export function prependRows(mount, batch, maxRows, toEl) {
  const frag = document.createDocumentFragment();
  for (let i = batch.length - 1; i >= 0; i--) frag.appendChild(toEl(batch[i]));
  mount.insertBefore(frag, mount.firstChild);
  while (mount.children.length > maxRows) mount.removeChild(mount.lastChild);
}
