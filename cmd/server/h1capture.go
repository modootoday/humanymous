package main

import (
	"bufio"
	"bytes"
	"io"
	"net"
	"strings"
	"time"
)

// h1capture.go peeks an HTTP/1.1 request's on-wire header-name ORDER before Go's
// net/http parses it into a map (which loses order — see HeaderInfo.OrderReliable /
// SoT-38 truth-debt, wargame R4). The consumed bytes are buffered and replayed so the
// standard http.Server serves the connection unchanged (the fingerproxy technique,
// mirroring h2capture.go).

// peekH1HeaderOrder reads up to the end of the request headers (CRLFCRLF), returns the
// lowercased header names in wire order plus a reader that replays everything consumed
// followed by the live stream. Bounded by a byte cap and a short deadline so a slowloris
// cannot stall the capture (the http.Server's ReadHeaderTimeout still governs the rest).
func peekH1HeaderOrder(conn net.Conn) ([]string, io.Reader, error) {
	conn.SetReadDeadline(time.Now().Add(750 * time.Millisecond))
	defer conn.SetReadDeadline(time.Time{}) // clear before handing to http.Server

	const maxHead = 1 << 16 // 64 KiB header cap for the peek (http.Server enforces its own)
	var buf bytes.Buffer
	br := bufio.NewReader(io.TeeReader(io.LimitReader(conn, maxHead), &buf))

	// Request line — consume it but do not parse (order starts at the first header).
	if _, err := br.ReadString('\n'); err != nil {
		return nil, replay(&buf, conn), err
	}
	var names []string
	for {
		line, err := br.ReadString('\n')
		if err != nil {
			return names, replay(&buf, conn), err
		}
		trimmed := strings.TrimRight(line, "\r\n")
		if trimmed == "" { // blank line: end of headers
			return names, replay(&buf, conn), nil
		}
		if i := strings.IndexByte(trimmed, ':'); i > 0 {
			names = append(names, strings.ToLower(strings.TrimSpace(trimmed[:i])))
		}
	}
}

// replayReadConn wraps a net.Conn, reading from a replay reader (buffered peeked bytes +
// live stream) while writing/closing on the underlying conn — the h1 analogue of replayConn.
type replayReadConn struct {
	net.Conn
	r io.Reader
}

func (c replayReadConn) Read(p []byte) (int, error) { return c.r.Read(p) }
