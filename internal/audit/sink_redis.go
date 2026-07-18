package audit

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"strconv"
	"sync"
	"time"
)

// sink_redis.go is a Redis Streams PROJECTION sink (SoT-32 Tier 1: hot buffer /
// live console tail). It speaks RESP directly over TCP — NO third-party client —
// so the core stays dependency-free. It is best-effort and non-blocking: it never
// blocks or fails the seal path (the WAL is the durability authority); if Redis is
// down or the buffer is full, records are dropped from the hot tail and counted.
//
// Each record is one XADD with the sealed record JSON as a single field, keyed by
// the chain seq as the stream id ("{seq}-0") so re-ships after a reconnect are
// idempotent, and trimmed with MAXLEN ~ so the hot tier stays bounded.

type RedisStreamSink struct {
	addr   string
	stream string
	maxLen int64

	buf     chan Record
	mu      sync.Mutex
	dropped uint64
	done    chan struct{}
	closed  bool
}

// NewRedisStreamSink starts a background shipper to addr (host:port). stream is
// the key (e.g. "audit:node-1"); maxLen bounds the retained hot window; bufSize
// bounds the in-process backlog before drop-and-count kicks in.
func NewRedisStreamSink(addr, stream string, maxLen int64, bufSize int) *RedisStreamSink {
	if bufSize <= 0 {
		bufSize = 4096
	}
	s := &RedisStreamSink{
		addr: addr, stream: stream, maxLen: maxLen,
		buf: make(chan Record, bufSize), done: make(chan struct{}),
	}
	go s.run()
	return s
}

// AppendRecord enqueues non-blocking. On a full buffer it drops the record from
// the hot tail and counts it (the durable copy is in the WAL).
func (s *RedisStreamSink) AppendRecord(_ context.Context, r Record) error {
	select {
	case s.buf <- r:
		return nil
	default:
		s.mu.Lock()
		s.dropped++
		s.mu.Unlock()
		return nil // best-effort projection: never fail the seal
	}
}

// Flush is a no-op ack point (the shipper drains asynchronously).
func (s *RedisStreamSink) Flush(_ context.Context) error { return nil }

// Close stops the shipper.
func (s *RedisStreamSink) Close() error {
	s.mu.Lock()
	if s.closed {
		s.mu.Unlock()
		return nil
	}
	s.closed = true
	s.mu.Unlock()
	close(s.done)
	return nil
}

// Dropped reports how many records were dropped from the hot tail under load.
func (s *RedisStreamSink) Dropped() uint64 {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.dropped
}

func (s *RedisStreamSink) run() {
	var conn net.Conn
	var br *bufio.Reader
	defer func() {
		if conn != nil {
			_ = conn.Close()
		}
	}()
	dial := func() bool {
		c, err := net.DialTimeout("tcp", s.addr, 3*time.Second)
		if err != nil {
			return false
		}
		conn, br = c, bufio.NewReader(c)
		return true
	}
	for {
		select {
		case <-s.done:
			return
		case r := <-s.buf:
			if conn == nil && !dial() {
				time.Sleep(time.Second) // backoff; the record is already durable in the WAL
				continue
			}
			if err := s.xadd(conn, br, r); err != nil {
				_ = conn.Close()
				conn = nil // reconnect on next record
			}
		}
	}
}

// xadd writes one XADD and reads the reply. Returns an error on any I/O or a RESP
// error reply so run() can reconnect.
func (s *RedisStreamSink) xadd(conn net.Conn, br *bufio.Reader, r Record) error {
	body, err := json.Marshal(r)
	if err != nil {
		return nil // unmarshalable record: skip, don't wedge the shipper
	}
	id := strconv.FormatUint(r.Seq, 10) + "-0"
	args := []string{"XADD", s.stream, "MAXLEN", "~", strconv.FormatInt(s.maxLen, 10), id, "record", string(body)}
	_ = conn.SetWriteDeadline(time.Now().Add(3 * time.Second))
	if _, err := conn.Write(respCommand(args)); err != nil {
		return err
	}
	_ = conn.SetReadDeadline(time.Now().Add(3 * time.Second))
	line, err := br.ReadString('\n')
	if err != nil {
		return err
	}
	if len(line) > 0 && line[0] == '-' { // RESP error reply
		return fmt.Errorf("redis: %s", line[1:])
	}
	return nil
}

// respCommand encodes a Redis command as a RESP array of bulk strings.
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
