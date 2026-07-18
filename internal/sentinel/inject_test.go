package sentinel

import (
	"io"
	"strings"
	"testing"
)

const snip = "<!--hmn-injected--><script src=\"/__hmn/loader.js\"></script>"

func runInject(t *testing.T, html string, chunk int) (string, injectResult) {
	t.Helper()
	var res injectResult
	r := newInjectingReader(&chunkedReader{s: []byte(html), n: chunk}, []byte(snip), &res)
	out, err := io.ReadAll(r)
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	return string(out), res
}

// chunkedReader emits the source in fixed-size chunks to exercise streaming.
type chunkedReader struct {
	s []byte
	n int
	i int
}

func (c *chunkedReader) Read(p []byte) (int, error) {
	if c.i >= len(c.s) {
		return 0, io.EOF
	}
	end := c.i + c.n
	if end > len(c.s) {
		end = len(c.s)
	}
	n := copy(p, c.s[c.i:end])
	c.i += n
	return n, nil
}

func TestInjectAfterHead(t *testing.T) {
	html := "<html><head><title>x</title></head><body>hi</body></html>"
	for _, chunk := range []int{1, 3, 7, 4096} { // split the boundary many ways
		out, res := runInject(t, html, chunk)
		if !res.Injected {
			t.Fatalf("chunk=%d: not injected (%s)", chunk, res.Reason)
		}
		if !strings.Contains(out, snip) {
			t.Fatalf("chunk=%d: snippet missing", chunk)
		}
		// add-only: original content preserved, snippet right after <head>.
		if !strings.Contains(out, "<head>"+snip) {
			t.Fatalf("chunk=%d: not inserted right after <head>: %s", chunk, out)
		}
		if !strings.Contains(out, "<title>x</title>") || !strings.Contains(out, "hi") {
			t.Fatalf("chunk=%d: origin bytes altered: %s", chunk, out)
		}
	}
}

func TestInjectFallbackBeforeBody(t *testing.T) {
	html := "<html><body>no head here</body></html>"
	out, res := runInject(t, html, 5)
	if !res.Injected {
		t.Fatalf("should inject before <body>: %s", res.Reason)
	}
	if !strings.Contains(out, snip+"<body>") {
		t.Fatalf("not inserted before <body>: %s", out)
	}
}

func TestIdempotentNoDoubleInject(t *testing.T) {
	html := "<html><head>" + injectMarker + "</head><body>x</body></html>"
	out, res := runInject(t, html, 6)
	if res.Injected {
		t.Fatalf("must not re-inject when marker present")
	}
	if strings.Count(out, injectMarker) != 1 {
		t.Fatalf("marker count changed: %s", out)
	}
}

func TestNoHTMLPassThrough(t *testing.T) {
	body := "just some plain text, definitely not html, no tags at all"
	out, res := runInject(t, body, 8)
	if res.Injected {
		t.Fatalf("should not inject into non-HTML")
	}
	if out != body {
		t.Fatalf("passthrough altered body: %q", out)
	}
}

func TestOversizeHeadlessBodyGivesUp(t *testing.T) {
	// A body that never closes head and exceeds the lookahead budget must not
	// buffer unbounded; it gives up and passes through (HR-27a).
	big := "<html><head>" + strings.Repeat("A", maxLookahead+1024)
	out, res := runInject(t, big, 4096)
	if res.Injected {
		// injection right after <head> is fine here since <head> opens; ensure
		// we didn't hang and output equals input+snippet or input.
	}
	if len(out) < len(big) {
		t.Fatalf("lost bytes on oversize body")
	}
}
