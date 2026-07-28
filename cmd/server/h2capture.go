package main

import (
	"bytes"
	"errors"
	"io"
	"net"
	"time"

	"github.com/modootoday/humanymous/internal/network"
	"golang.org/x/net/http2"
	"golang.org/x/net/http2/hpack"
)

// h2capture.go peeks the client's HTTP/2 preface + SETTINGS/WINDOW_UPDATE frames
// and the first HEADERS frame's pseudo-header order, computing the real Akamai
// HTTP/2 fingerprint (SoT-02 §HTTP/2). The consumed bytes are buffered and
// replayed so the standard http2 server can serve the connection unchanged
// (the fingerproxy technique, plan/02 §3.2).

var h2Preface = []byte("PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")

// peekH2 reads from conn until the first HEADERS frame, returns the fingerprint
// and a reader that replays everything consumed followed by the live stream.
func peekH2(conn net.Conn) (*network.H2Fingerprint, io.Reader, error) {
	conn.SetReadDeadline(time.Now().Add(750 * time.Millisecond))
	defer conn.SetReadDeadline(time.Time{}) // clear before handing to ServeConn

	var buf bytes.Buffer
	tee := io.TeeReader(conn, &buf)

	// Consume + verify the connection preface.
	pf := make([]byte, len(h2Preface))
	if _, err := io.ReadFull(tee, pf); err != nil {
		return nil, replay(&buf, conn), err
	}
	if !bytes.Equal(pf, h2Preface) {
		return nil, replay(&buf, conn), errors.New("h2: bad preface")
	}

	fr := http2.NewFramer(io.Discard, tee)
	fr.ReadMetaHeaders = nil
	fp := &network.H2Fingerprint{}
	dec := hpack.NewDecoder(4096, nil)

	for i := 0; i < 16; i++ { // bounded: browsers send SETTINGS+WU then HEADERS early
		f, err := fr.ReadFrame()
		if err != nil {
			return fp, replay(&buf, conn), err
		}
		switch frame := f.(type) {
		case *http2.SettingsFrame:
			frame.ForeachSetting(func(s http2.Setting) error {
				fp.Settings = append(fp.Settings, network.H2Setting{ID: uint16(s.ID), Value: s.Val})
				return nil
			})
		case *http2.WindowUpdateFrame:
			if fp.WindowUpdate == 0 {
				fp.WindowUpdate = frame.Increment
			}
		case *http2.PriorityFrame:
			p := frame.PriorityParam
			fp.Priorities = append(fp.Priorities,
				itoaFrame(frame.StreamID, p.Exclusive, p.StreamDep, p.Weight))
		case *http2.HeadersFrame:
			fields, _ := dec.DecodeFull(frame.HeaderBlockFragment())
			for _, hf := range fields {
				if len(hf.Name) > 0 && hf.Name[0] == ':' {
					if code := pseudoCode(hf.Name); code != "" {
						fp.PseudoOrder = append(fp.PseudoOrder, code)
					}
					continue
				}
				// Regular header names in wire order (h2 mandates lowercase) — the
				// h2 analogue of the h1 header-order capture (SoT-02 / R4).
				fp.HeaderOrder = append(fp.HeaderOrder, hf.Name)
			}
			return fp, replay(&buf, conn), nil // first HEADERS -> done
		}
	}
	return fp, replay(&buf, conn), nil
}

// replay returns a reader that yields the buffered bytes then the live conn.
func replay(buf *bytes.Buffer, conn net.Conn) io.Reader {
	return io.MultiReader(bytes.NewReader(buf.Bytes()), conn)
}

// replayConn wraps a net.Conn, reading from a replay reader (buffered peeked
// bytes + live stream) while writing/closing on the underlying conn. It passively
// feeds read bytes to a frame monitor for HTTP/2 DoS detection (SoT-17).
type replayConn struct {
	net.Conn
	r       io.Reader
	mon     *frameMonitor
	onAbuse func(string) // called once with the abuse signal id
}

func (c replayConn) Read(p []byte) (int, error) {
	n, err := c.r.Read(p)
	if n > 0 && c.mon != nil && c.mon.abuse == "" {
		c.mon.feed(p[:n])
		if c.mon.abuse != "" && c.onAbuse != nil {
			c.onAbuse(c.mon.abuse)
		}
	}
	return n, err
}

func pseudoCode(name string) string {
	switch name {
	case ":method":
		return "m"
	case ":authority":
		return "a"
	case ":scheme":
		return "s"
	case ":path":
		return "p"
	}
	return ""
}

func itoaFrame(stream uint32, excl bool, dep uint32, weight uint8) string {
	e := "0"
	if excl {
		e = "1"
	}
	return uitoa(stream) + ":" + e + ":" + uitoa(dep) + ":" + uitoa(uint32(weight))
}

func uitoa(v uint32) string {
	if v == 0 {
		return "0"
	}
	var b [10]byte
	i := len(b)
	for v > 0 {
		i--
		b[i] = byte('0' + v%10)
		v /= 10
	}
	return string(b[i:])
}
