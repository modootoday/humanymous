// Package redis is a dependency-free, synchronous request/reply RESP2 client for
// the small command set humanymous needs to share security state across nodes
// (PLAN-08 R1): GET/SET/DEL/EXPIRE/SCAN for the ban + verdict ledgers and EVAL for
// the atomic GCRA rate limiter. It is intentionally minimal — a single
// mutex-guarded connection with reconnect-on-error, no pooling or pipelining —
// matching the reference-implementation ethos. The audit projection already speaks
// fire-and-forget RESP (internal/audit/sink_redis.go); this adds the reply-parsing
// half so a caller can read a value back.
package redis

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"net"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Client is a synchronous RESP2 client over one guarded TCP connection, with a
// circuit breaker so a coordinator outage degrades FAST (fail immediately for a
// cooldown) instead of stalling every request on the dial/read timeout — the caller
// then serves from its local view. This keeps the no-stall / no-lockout ethos.
type Client struct {
	addr      string
	timeout   time.Duration
	cooldown  time.Duration
	slow      time.Duration // latency budget: a command slower than this trips the breaker
	user      string        // optional ACL username (AUTH)
	pass      string        // optional password (AUTH); sent on connect
	mu        sync.Mutex
	conn      net.Conn
	br        *bufio.Reader
	openUntil time.Time // breaker: fail fast until this time (zero = closed)
}

// Reply-size caps so a malicious or compromised coordinator cannot force an
// unbounded allocation (OOM / panic DoS) via an oversized declared length. Larger
// than any value humanymous stores; SCAN arrays stay well under the element cap.
const (
	maxBulkLen  = 32 << 20 // 32 MiB per bulk string
	maxArrayLen = 1 << 20  // 1,048,576 array elements
	maxDepth    = 8        // nesting cap: our deepest reply is SCAN (depth 2); a
	// compromised coordinator otherwise nests arrays arbitrarily (*1\r\n…) to drive an
	// unrecoverable stack-overflow throw (the size caps guard WIDTH only).
)

// ErrBreakerOpen is returned while the circuit breaker is open (recent failure);
// the caller should serve from its local fallback without waiting.
var ErrBreakerOpen = errors.New("redis: circuit breaker open")

// New returns a client for addr (host:port). The connection is established lazily
// on the first command and re-established after any I/O error.
func New(addr string) *Client {
	return &Client{addr: addr, timeout: 3 * time.Second, cooldown: 2 * time.Second, slow: 750 * time.Millisecond}
}

// SetAuth configures an optional AUTH credential sent on every (re)connect. An empty
// password disables AUTH. user may be empty for a legacy password-only AUTH.
func (c *Client) SetAuth(user, pass string) {
	c.mu.Lock()
	c.user, c.pass = user, pass
	c.mu.Unlock()
}

// Reply is a parsed RESP2 reply. Exactly one shape is populated: Int (integer),
// Str (simple or bulk string), Array (array), or Nil (a null bulk/array). A RESP
// error reply is surfaced as a Go error from Do, never as a Reply.
type Reply struct {
	Int   int64
	Str   string
	Array []Reply
	Nil   bool
}

// Do sends one command and returns its parsed reply. Any I/O error drops the
// connection so the next call reconnects; the caller decides how to degrade.
func (c *Client) Do(args ...string) (Reply, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	now := time.Now()
	if !c.openUntil.IsZero() && now.Before(c.openUntil) {
		return Reply{}, ErrBreakerOpen // fail fast: don't stall on a known-down coordinator
	}
	if err := c.ensure(); err != nil {
		return Reply{}, c.trip(err)
	}
	_ = c.conn.SetWriteDeadline(time.Now().Add(c.timeout))
	if _, err := c.conn.Write(respCommand(args)); err != nil {
		c.drop()
		return Reply{}, c.trip(err)
	}
	_ = c.conn.SetReadDeadline(time.Now().Add(c.timeout))
	rep, err := readReply(c.br, 0)
	if err != nil {
		c.drop()
		return Reply{}, c.trip(err)
	}
	// Latency budget: a command that SUCCEEDS but is slow (a degraded coordinator, not a
	// dead one) would otherwise be invisible to the breaker, so every request keeps paying
	// that latency serialized behind the single connection's mutex — a wedge (deep-review).
	// Trip the breaker on slow-success too: the reply is still returned, but the next
	// cooldown window fast-fails to the caller's local view instead of stalling. The conn
	// stays up (no drop) so a transient blip self-heals after the cooldown.
	if c.slow > 0 && time.Since(now) > c.slow {
		c.openUntil = time.Now().Add(c.cooldown)
		return rep, nil
	}
	c.openUntil = time.Time{} // success: close the breaker
	return rep, nil
}

// trip opens the breaker for the cooldown window and returns the causing error.
func (c *Client) trip(err error) error {
	c.openUntil = time.Now().Add(c.cooldown)
	return err
}

// Close releases the connection.
func (c *Client) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.drop()
	return nil
}

func (c *Client) ensure() error {
	if c.conn != nil {
		return nil
	}
	conn, err := net.DialTimeout("tcp", c.addr, c.timeout)
	if err != nil {
		return err
	}
	c.conn, c.br = conn, bufio.NewReader(conn)
	// Authenticate the fresh connection if a credential is configured. A failed AUTH
	// drops the connection and surfaces the error (the caller then serves local).
	if c.pass != "" {
		args := []string{"AUTH", c.pass}
		if c.user != "" {
			args = []string{"AUTH", c.user, c.pass}
		}
		_ = c.conn.SetWriteDeadline(time.Now().Add(c.timeout))
		if _, err := c.conn.Write(respCommand(args)); err != nil {
			c.drop()
			return err
		}
		_ = c.conn.SetReadDeadline(time.Now().Add(c.timeout))
		if _, err := readReply(c.br, 0); err != nil { // RESP error (e.g. WRONGPASS) → err
			c.drop()
			return err
		}
	}
	return nil
}

func (c *Client) drop() {
	if c.conn != nil {
		_ = c.conn.Close()
		c.conn, c.br = nil, nil
	}
}

// readReply parses one RESP2 reply from br, refusing nesting past maxDepth.
func readReply(br *bufio.Reader, depth int) (Reply, error) {
	if depth > maxDepth {
		return Reply{}, fmt.Errorf("redis: reply nesting exceeds cap %d", maxDepth)
	}
	line, err := br.ReadString('\n')
	if err != nil {
		return Reply{}, err
	}
	if len(line) < 3 {
		return Reply{}, fmt.Errorf("redis: short reply %q", line)
	}
	body := strings.TrimRight(line[1:], "\r\n")
	switch line[0] {
	case '+': // simple string
		return Reply{Str: body}, nil
	case '-': // error
		return Reply{}, fmt.Errorf("redis: %s", body)
	case ':': // integer
		n, err := strconv.ParseInt(body, 10, 64)
		return Reply{Int: n}, err
	case '$': // bulk string
		n, err := strconv.Atoi(body)
		if err != nil {
			return Reply{}, err
		}
		if n < 0 {
			return Reply{Nil: true}, nil
		}
		if n > maxBulkLen {
			return Reply{}, fmt.Errorf("redis: bulk length %d exceeds cap %d", n, maxBulkLen)
		}
		buf := make([]byte, n+2) // payload + CRLF
		if _, err := io.ReadFull(br, buf); err != nil {
			return Reply{}, err
		}
		return Reply{Str: string(buf[:n])}, nil
	case '*': // array
		n, err := strconv.Atoi(body)
		if err != nil {
			return Reply{}, err
		}
		if n < 0 {
			return Reply{Nil: true}, nil
		}
		if n > maxArrayLen {
			return Reply{}, fmt.Errorf("redis: array length %d exceeds cap %d", n, maxArrayLen)
		}
		arr := make([]Reply, n)
		for i := 0; i < n; i++ {
			if arr[i], err = readReply(br, depth+1); err != nil {
				return Reply{}, err
			}
		}
		return Reply{Array: arr}, nil
	default:
		return Reply{}, fmt.Errorf("redis: unknown reply type %q", line[0])
	}
}

// respCommand encodes a command as a RESP array of bulk strings.
func respCommand(args []string) []byte {
	buf := make([]byte, 0, 32)
	buf = append(buf, '*')
	buf = strconv.AppendInt(buf, int64(len(args)), 10)
	buf = append(buf, '\r', '\n')
	for _, a := range args {
		buf = append(buf, '$')
		buf = strconv.AppendInt(buf, int64(len(a)), 10)
		buf = append(buf, '\r', '\n')
		buf = append(buf, a...)
		buf = append(buf, '\r', '\n')
	}
	return buf
}
