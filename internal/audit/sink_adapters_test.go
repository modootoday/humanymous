package audit

import (
	"bufio"
	"context"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestRespCommandEncoding(t *testing.T) {
	got := string(respCommand([]string{"XADD", "s", "1-0"}))
	want := "*3\r\n$4\r\nXADD\r\n$1\r\ns\r\n$3\r\n1-0\r\n"
	if got != want {
		t.Fatalf("respCommand:\n got %q\nwant %q", got, want)
	}
}

// End-to-end over a fake Redis: the shipper connects and issues an XADD carrying
// the stream, MAXLEN, the seq-derived id, and the record field.
func TestRedisStreamSinkXADD(t *testing.T) {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer ln.Close()

	var mu sync.Mutex
	var got string
	ready := make(chan struct{})
	go func() {
		c, err := ln.Accept()
		if err != nil {
			return
		}
		defer c.Close()
		br := bufio.NewReader(c)
		// Read one RESP array command (*8 ... 8 bulk strings for our XADD).
		hdr, _ := br.ReadString('\n') // *8\r\n
		n := 0
		for _, ch := range hdr[1:] {
			if ch >= '0' && ch <= '9' {
				n = n*10 + int(ch-'0')
			}
		}
		var sb strings.Builder
		for i := 0; i < n; i++ {
			br.ReadString('\n')            // $len
			line, _ := br.ReadString('\n') // value\r\n
			sb.WriteString(strings.TrimRight(line, "\r\n"))
			sb.WriteByte(' ')
		}
		mu.Lock()
		got = sb.String()
		mu.Unlock()
		c.Write([]byte("+OK\r\n"))
		close(ready)
	}()

	s := NewRedisStreamSink(ln.Addr().String(), "audit:n1", 1000, 16)
	defer s.Close()
	r := sampleRecord(0)
	r.Seq = 7
	_ = s.AppendRecord(context.Background(), r)

	select {
	case <-ready:
	case <-time.After(3 * time.Second):
		t.Fatal("redis shipper did not send XADD")
	}
	mu.Lock()
	cmd := got
	mu.Unlock()
	for _, want := range []string{"XADD", "audit:n1", "MAXLEN", "~", "1000", "7-0", "record"} {
		if !strings.Contains(cmd, want) {
			t.Errorf("XADD missing %q in %q", want, cmd)
		}
	}
}

// A full buffer drops + counts rather than blocking the seal.
func TestRedisStreamSinkDropsWhenFull(t *testing.T) {
	// unroutable addr so nothing drains; tiny buffer.
	s := &RedisStreamSink{addr: "203.0.113.1:6379", stream: "x", maxLen: 10, buf: make(chan Record, 1), done: make(chan struct{})}
	// Do NOT start run(); fill the buffer past capacity.
	_ = s.AppendRecord(context.Background(), sampleRecord(0)) // fills the 1 slot
	_ = s.AppendRecord(context.Background(), sampleRecord(1)) // dropped
	_ = s.AppendRecord(context.Background(), sampleRecord(2)) // dropped
	if s.Dropped() != 2 {
		t.Fatalf("Dropped = %d, want 2", s.Dropped())
	}
}

// The ClickHouse sink POSTs an INSERT ... FORMAT JSONEachRow batch with the
// records as the body.
func TestCHSinkInsert(t *testing.T) {
	type capture struct {
		query string
		body  string
	}
	got := make(chan capture, 1)
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		buf := make([]byte, 1<<16)
		n, _ := r.Body.Read(buf)
		select {
		case got <- capture{query: r.URL.RawQuery, body: string(buf[:n])}:
		default:
		}
		w.WriteHeader(200)
	}))
	defer srv.Close()

	s := NewCHSink(srv.URL, "audit_log", 100, 50*time.Millisecond, 1000)
	defer s.Close()
	r := sampleRecord(0)
	r.Seq = 42
	_ = s.AppendRecord(context.Background(), r)
	if err := s.Flush(context.Background()); err != nil {
		t.Fatalf("flush: %v", err)
	}

	select {
	case c := <-got:
		if !strings.Contains(c.query, "INSERT+INTO+audit_log") && !strings.Contains(c.query, "INSERT%20INTO%20audit_log") {
			t.Errorf("query missing INSERT INTO audit_log: %s", c.query)
		}
		if !strings.Contains(c.query, "JSONEachRow") {
			t.Errorf("query missing JSONEachRow: %s", c.query)
		}
		if !strings.Contains(c.body, "\"seq\":42") {
			t.Errorf("body missing record: %s", c.body)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("clickhouse sink did not POST an insert")
	}
}

// PLAN-08 backlog: an unsafe projection table name is rejected (falls back to the safe default).
func TestCHTableNameValidation(t *testing.T) {
	if validTableName.MatchString("audit_log; DROP TABLE x") {
		t.Error("an injection-y table name must NOT be accepted")
	}
	if !validTableName.MatchString("audit_log") {
		t.Error("a normal identifier must be accepted")
	}
}
