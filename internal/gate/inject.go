// Package gate implements the humanymous Gate reverse-proxy security layer
// (SoT-19, SoT-20, SoT-21): a TLS-terminating reverse proxy that streams the detection
// bundle into upstream HTML, enforces L7 verdicts at the edge, and emits every
// decision into the tamper-evident audit log before it takes effect.
package gate

import (
	"bytes"
	"io"
)

// inject.go is the streaming, single-pass HTML bundle injector (SoT-20). It
// injects the detection snippet exactly once at the first head boundary using
// bounded look-ahead, is idempotent (a marker prevents double injection), and is
// ADD-ONLY — it never rewrites or narrows origin bytes, it only inserts one tag
// block. Callers force Accept-Encoding: identity upstream so the body is plain.

// injectMarker makes injection idempotent: if it is already present we pass
// through untouched (SoT-20 §2).
const injectMarker = "<!--hmn-injected-->"

// maxLookahead bounds how far we buffer while hunting for the insertion point
// before falling back, so a headless/never-closing-head body cannot make us
// buffer without limit (SoT-20 §8, HR-27a buffer_overrun guard).
const maxLookahead = 64 << 10 // 64 KiB

// injectResult reports what the injector did (feeds the audit event).
type injectResult struct {
	Injected bool
	Reason   string // when not injected: already_present | point_not_found | too_large
}

// injectingReader wraps an upstream body reader and yields the same bytes with
// the snippet inserted once at the first head boundary. It never buffers more
// than maxLookahead before deciding.
type injectingReader struct {
	src        io.Reader
	snippet    []byte
	buf        []byte // pending output (post-injection)
	pre        []byte // look-ahead accumulator (pre-injection)
	done       bool   // injection resolved (either inserted or gave up)
	srcEOF     bool
	pendingErr error // non-EOF upstream error to surface after buffered output
	res        *injectResult
}

// newInjectingReader builds a reader that inserts snippet into the HTML stream.
// The result pointer is updated once the decision is made.
func newInjectingReader(src io.Reader, snippet []byte, res *injectResult) *injectingReader {
	return &injectingReader{src: src, snippet: snippet, res: res}
}

func (r *injectingReader) Read(p []byte) (int, error) {
	// Serve any already-decided output first.
	if len(r.buf) > 0 {
		n := copy(p, r.buf)
		r.buf = r.buf[n:]
		return n, nil
	}
	if r.done {
		return r.src.Read(p) // pass-through remainder
	}
	// Accumulate look-ahead until we can decide.
	for !r.done {
		// An existing marker (always sits right after a prior insertion point)
		// means the page is already injected — pass through untouched.
		if bytes.Contains(r.pre, []byte(injectMarker)) {
			r.resolve(0, false, "already_present")
			break
		}
		if idx := findInsertionPoint(r.pre); idx >= 0 {
			// Confirm the marker doesn't immediately follow the point; wait for
			// those bytes before deciding (unless upstream ended).
			if len(r.pre) < idx+len(injectMarker) && !r.srcEOF {
				// fall through to read more
			} else if bytes.HasPrefix(r.pre[idx:], []byte(injectMarker)) {
				r.resolve(0, false, "already_present")
				break
			} else {
				r.resolve(idx, true, "")
				break
			}
		}
		if len(r.pre) >= maxLookahead {
			r.resolve(0, false, "point_not_found")
			break
		}
		tmp := make([]byte, 8<<10)
		n, err := r.src.Read(tmp)
		r.pre = append(r.pre, tmp[:n]...)
		if err == io.EOF {
			r.srcEOF = true
			// EOF before finding a point: inject at a fallback if this looks
			// like HTML, else pass through un-injected.
			if idx := findInsertionPoint(r.pre); idx >= 0 {
				r.resolve(idx, true, "")
			} else {
				r.resolve(0, false, "point_not_found")
			}
			break
		}
		if err != nil {
			r.resolve(0, false, "point_not_found")
			r.pendingErr = err
			break
		}
	}
	if len(r.buf) > 0 {
		n := copy(p, r.buf)
		r.buf = r.buf[n:]
		return n, nil
	}
	if r.pendingErr != nil {
		return 0, r.pendingErr
	}
	if r.srcEOF {
		return 0, io.EOF
	}
	return r.src.Read(p)
}

// resolve finalizes the injection decision, staging the output buffer.
func (r *injectingReader) resolve(idx int, inject bool, reason string) {
	if inject {
		out := make([]byte, 0, len(r.pre)+len(r.snippet))
		out = append(out, r.pre[:idx]...)
		out = append(out, r.snippet...)
		out = append(out, r.pre[idx:]...)
		r.buf = out
		*r.res = injectResult{Injected: true}
	} else {
		r.buf = r.pre
		*r.res = injectResult{Injected: false, Reason: reason}
	}
	r.pre = nil
	r.done = true
}

// findInsertionPoint returns the byte offset to insert at (right after <head...>
// if present, else right before </head>, else before <body...>, else -1). It is
// case-insensitive. Add-only: we never modify the surrounding bytes (SoT-20 §6).
func findInsertionPoint(b []byte) int {
	lower := bytes.ToLower(b)
	// after the first <head ...> opening tag
	if i := bytes.Index(lower, []byte("<head")); i >= 0 {
		if gt := bytes.IndexByte(b[i:], '>'); gt >= 0 {
			return i + gt + 1
		}
		return -1 // <head not yet closed in this chunk; wait for more
	}
	// before </head>
	if i := bytes.Index(lower, []byte("</head>")); i >= 0 {
		return i
	}
	// before <body ...>
	if i := bytes.Index(lower, []byte("<body")); i >= 0 {
		return i
	}
	return -1
}
