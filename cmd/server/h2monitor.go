package main

import "encoding/binary"

// h2monitor.go passively counts HTTP/2 frame types as the decrypted byte stream
// passes to the h2 server, detecting the destructive stream-reset DoS families
// (SoT-17): Rapid Reset (CVE-2023-44487, client RST_STREAM flood), CONTINUATION
// Flood, and MadeYouReset (CVE-2025-8671) — where protocol-violating control
// frames (WINDOW_UPDATE increment 0 / PRIORITY with wrong length / frames on
// half-closed streams) make the SERVER emit RST_STREAM, bypassing defenses that
// count only client resets. The monitor parses just the 9-byte frame headers
// (no HPACK), so it never interferes with serving.

// HTTP/2 frame type codes (RFC 9113 §6).
const (
	ftData         = 0x0
	ftHeaders      = 0x1
	ftPriority     = 0x2
	ftRSTStream    = 0x3
	ftSettings     = 0x4
	ftPing         = 0x6
	ftWindowUpdate = 0x8
	ftContinuation = 0x9
)

// Per-connection abuse thresholds (research baselines, generous to avoid FP).
const (
	maxRSTStream    = 15  // Rapid Reset / MadeYouReset resets per connection
	maxContinuation = 20  // CONTINUATION flood
	maxProtoErr     = 10  // MadeYouReset-triggering protocol violations
	maxControl      = 300 // control-frame flood (PING/PRIORITY/SETTINGS)
)

// frameMonitor accumulates bytes and counts frame types.
type frameMonitor struct {
	buf         []byte
	prefaceSkip int // remaining preface bytes to skip (24-byte client preface)
	rst         int
	cont        int
	protoErr    int
	control     int
	abuse       string // set once when a threshold is crossed
}

func newFrameMonitor() *frameMonitor { return &frameMonitor{prefaceSkip: 24} }

// feed consumes bytes and updates counters/abuse. It is best-effort: any parse
// desync just stops counting (never affects the served connection).
func (m *frameMonitor) feed(p []byte) {
	if m.abuse != "" {
		return
	}
	m.buf = append(m.buf, p...)
	// Skip the connection preface first.
	if m.prefaceSkip > 0 {
		n := m.prefaceSkip
		if n > len(m.buf) {
			n = len(m.buf)
		}
		m.buf = m.buf[n:]
		m.prefaceSkip -= n
	}
	for len(m.buf) >= 9 {
		length := int(m.buf[0])<<16 | int(m.buf[1])<<8 | int(m.buf[2])
		ftype := m.buf[3]
		if len(m.buf) < 9+length {
			break // wait for the full frame payload
		}
		payload := m.buf[9 : 9+length]
		m.classify(ftype, length, payload)
		m.buf = m.buf[9+length:]
		if m.abuse != "" {
			m.buf = nil
			return
		}
	}
}

func (m *frameMonitor) classify(ftype byte, length int, payload []byte) {
	switch ftype {
	case ftRSTStream:
		m.rst++
		if m.rst > maxRSTStream {
			m.abuse = "l5.h2dos.rapid_reset"
		}
	case ftContinuation:
		m.cont++
		if m.cont > maxContinuation {
			m.abuse = "l5.h2dos.continuation_flood"
		}
	case ftWindowUpdate:
		m.control++
		// MadeYouReset trigger: WINDOW_UPDATE increment 0 (PROTOCOL_ERROR).
		if length == 4 && binary.BigEndian.Uint32(payload)&0x7fffffff == 0 {
			m.protoErr++
		}
	case ftPriority:
		m.control++
		// MadeYouReset trigger: PRIORITY frame length must be exactly 5.
		if length != 5 {
			m.protoErr++
		}
	case ftPing, ftSettings:
		m.control++
	case ftData, ftHeaders:
		// request-carrying frames; not abuse counters.
	}
	if m.protoErr > maxProtoErr {
		m.abuse = "l5.h2dos.rapid_reset" // MadeYouReset -> server reset flood
	}
	if m.control > maxControl {
		m.abuse = "l5.h2dos.rapid_reset"
	}
}
