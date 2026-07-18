package audit

import (
	"context"
	"crypto/ed25519"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"sync"
)

// chain.go implements the append-only per-node hash chain with a secondary
// per-record HMAC and periodic asymmetric (Ed25519) Signed-Tree-Head checkpoints
// (SoT-18 §4). The writer holds the HMAC key (symmetric, a secondary layer) but
// NOT the Ed25519 private key in the real deployment (HSM-held); here we keep it
// in-process for the reference implementation and expose only the public key for
// verification, mirroring the trust split.

// Log is a single node's append-only audit chain.
type Log struct {
	mu       sync.Mutex
	nodeID   string
	hmacKey  []byte             // secondary integrity (HSM/KMS in prod)
	signPriv ed25519.PrivateKey // STH signing (HSM in prod; verifier only needs pub)
	witness  *Witness           // independent co-signer (SoT-28 WS8), optional
	records  []Record
	prevHash string
	seq      uint64
	// checkpoint policy
	everyN      uint64
	checkpoints []Checkpoint
	sinceCP     uint64
	// SoT-32 durability: with a WAL the full chain lives on disk (authority) and
	// `records` is a bounded in-memory window; without one, behavior is unchanged
	// (ephemeral, unbounded in-mem). Projections are async, best-effort.
	wal         *WALSink
	ringCap     int
	projections []RecordSink
}

const defaultRingCap = 4096

// Checkpoint is a Signed Tree Head anchored externally (SoT-18 §4). Auditors
// verify the Ed25519 signature with the public key alone — no forging power.
type Checkpoint struct {
	NodeID     string `json:"node_id"`
	TreeSize   uint64 `json:"tree_size"` // number of records covered
	Root       string `json:"root"`      // hex chained root == last record_hash
	PrevCP     string `json:"prev_checkpoint_hash"`
	Sig        string `json:"sig"`         // hex Ed25519 (writer key) over canonical STH bytes
	WitnessSig string `json:"witness_sig"` // hex Ed25519 (independent witness key), SoT-28 WS8
}

// Config configures a Log.
type Config struct {
	NodeID          string
	HMACKey         []byte
	CheckpointEvery uint64   // records per Signed Tree Head (0 -> default 16)
	Witness         *Witness // optional independent co-signer (SoT-28 WS8)
	SigningSeed     []byte   // optional persisted Ed25519 seed (SoT-28 WS8); else generated
	// SoT-32: durable authority WAL (nil = ephemeral in-mem, unchanged). RingCap
	// bounds the in-memory window when a WAL is set (0 -> default). Projections are
	// async replayable sinks (Redis/ClickHouse).
	WAL         *WALSink
	RingCap     int
	Projections []RecordSink
}

// NewLog creates a node audit chain. If cfg.SigningSeed is a 32-byte persisted
// seed the STH key is stable across restarts (SoT-28 WS8); otherwise a fresh
// keypair is generated (ephemeral node identity).
func NewLog(cfg Config) *Log {
	var priv ed25519.PrivateKey
	if len(cfg.SigningSeed) == ed25519.SeedSize {
		priv = ed25519.NewKeyFromSeed(cfg.SigningSeed)
	} else {
		_, priv, _ = ed25519.GenerateKey(nil)
	}
	n := cfg.CheckpointEvery
	if n == 0 {
		n = 16
	}
	l := &Log{
		nodeID:      cfg.NodeID,
		hmacKey:     cfg.HMACKey,
		signPriv:    priv,
		witness:     cfg.Witness,
		everyN:      n,
		wal:         cfg.WAL,
		ringCap:     cfg.RingCap,
		projections: cfg.Projections,
	}
	if l.wal != nil {
		if l.ringCap <= 0 {
			l.ringCap = defaultRingCap
		}
		l.replayWAL()
	}
	return l
}

// replayWAL restores chain state from the durable WAL on boot (SoT-32): the full
// record set is on disk, so seq/prev_hash/checkpoints continue where they left
// off and `records` is repopulated as the bounded window. A corrupt/unreadable
// authority fails closed (panic) — the process must not run without its audit.
func (l *Log) replayWAL() {
	recs, err := l.wal.ReadAll()
	if err != nil {
		panic("audit: WAL replay failed (fail-closed): " + err.Error())
	}
	l.checkpoints, _ = l.wal.ReadCheckpoints()
	if len(recs) == 0 {
		return
	}
	last := recs[len(recs)-1]
	l.seq = last.Seq
	l.prevHash = last.RecordHash
	start := 0
	if len(recs) > l.ringCap {
		start = len(recs) - l.ringCap
	}
	l.records = append([]Record(nil), recs[start:]...)
	var lastCP uint64
	if n := len(l.checkpoints); n > 0 {
		lastCP = l.checkpoints[n-1].TreeSize
	}
	l.sinceCP = l.seq - lastCP
}

// WitnessPublicKey returns the co-signer's verification key ("" via nil if no
// witness is configured).
func (l *Log) WitnessPublicKey() ed25519.PublicKey {
	if l.witness == nil {
		return nil
	}
	return l.witness.Public()
}

// PublicKey returns the STH verification key (what an auditor holds).
func (l *Log) PublicKey() ed25519.PublicKey {
	return l.signPriv.Public().(ed25519.PublicKey)
}

// Append seals a record into the chain: assigns seq, links prev_hash, computes
// record_hash over the frozen canonical form, adds the HMAC, and emits a
// checkpoint every N records. Returns the sealed record. Callers MUST have
// pseudonymized identifiers already (no raw PII).
func (l *Log) Append(r Record) Record {
	l.mu.Lock()
	defer l.mu.Unlock()

	seq := l.seq + 1
	r.Seq = seq
	r.NodeID = l.nodeID
	if r.EventVer == "" {
		r.EventVer = EventSchemaVersion
	}
	if r.Rules == nil {
		r.Rules = []string{}
	}
	if r.TopSignals == nil {
		r.TopSignals = []Signal{}
	}
	r.PrevHash = l.prevHash
	r.RecordHash = computeRecordHash(&r, l.prevHash)
	mac := hmac.New(sha256.New, l.hmacKey)
	mac.Write([]byte(r.RecordHash))
	r.HMAC = hex.EncodeToString(mac.Sum(nil))

	// SoT-32 durability-first: persist to the authority WAL BEFORE mutating any
	// chain state, so a WAL failure leaves seq/prev_hash untouched (no gap) and
	// fails closed — EmitAndAct's side effect never fires for an un-durable record.
	if l.wal != nil {
		if err := l.wal.AppendRecord(context.Background(), r); err != nil {
			panic("audit: WAL append failed (fail-closed): " + err.Error())
		}
	}

	// Commit chain state.
	l.seq = seq
	l.records = append(l.records, r)
	l.prevHash = r.RecordHash
	l.sinceCP++

	// With a WAL, the full chain is on disk; bound the in-memory window.
	if l.wal != nil && l.ringCap > 0 && len(l.records) > 2*l.ringCap {
		l.records = append([]Record(nil), l.records[len(l.records)-l.ringCap:]...)
	}
	// Async projections (Redis/ClickHouse): best-effort, never block/fail the seal.
	for _, p := range l.projections {
		_ = p.AppendRecord(context.Background(), r)
	}

	if l.sinceCP >= l.everyN {
		l.checkpointLocked()
	}
	return r
}

// checkpointLocked signs the current tree head. Caller holds l.mu.
func (l *Log) checkpointLocked() {
	prev := ""
	if n := len(l.checkpoints); n > 0 {
		prev = l.checkpoints[n-1].Sig
	}
	cp := Checkpoint{
		NodeID:   l.nodeID,
		TreeSize: l.seq,
		Root:     l.prevHash,
		PrevCP:   prev,
	}
	cp.Sig = hex.EncodeToString(ed25519.Sign(l.signPriv, sthBytes(cp)))
	// Independent witness counter-signature (SoT-28 WS8): the witness only signs a
	// monotonically-growing tree, so a history rewrite cannot obtain a valid one.
	if l.witness != nil {
		if wsig, err := l.witness.CounterSign(cp); err == nil {
			cp.WitnessSig = wsig
		}
	}
	l.checkpoints = append(l.checkpoints, cp)
	l.sinceCP = 0
	// Persist the STH so the checkpoint chain survives a restart (best-effort; the
	// records in the WAL are the authority and can re-anchor if this is lost).
	if l.wal != nil {
		_ = l.wal.AppendCheckpoint(cp)
	}
}

// Checkpoint forces a Signed Tree Head (e.g. on graceful shutdown).
func (l *Log) Checkpoint() Checkpoint {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.checkpointLocked()
	return l.checkpoints[len(l.checkpoints)-1]
}

// Records returns the full chain (read-only export/verify). With a WAL the full
// set is read from disk (the in-memory copy is only a bounded window); without a
// WAL it is the in-memory snapshot (unchanged behavior).
func (l *Log) Records() []Record {
	l.mu.Lock()
	defer l.mu.Unlock()
	if l.wal == nil {
		return append([]Record(nil), l.records...)
	}
	recs, err := l.wal.ReadAll()
	if err != nil {
		return nil
	}
	return recs
}

// Recent returns up to the last n sealed records from the in-memory window (a
// fast tail read for the live console feed; bounded to the ring when a WAL is on).
func (l *Log) Recent(n int) []Record {
	l.mu.Lock()
	defer l.mu.Unlock()
	if n <= 0 || n > len(l.records) {
		n = len(l.records)
	}
	return append([]Record(nil), l.records[len(l.records)-n:]...)
}

// Checkpoints returns a snapshot of the anchored Signed Tree Heads.
func (l *Log) Checkpoints() []Checkpoint {
	l.mu.Lock()
	defer l.mu.Unlock()
	return append([]Checkpoint(nil), l.checkpoints...)
}

// Len returns the total number of sealed records (== seq; with a WAL the
// in-memory window may hold fewer than this).
func (l *Log) Len() int {
	l.mu.Lock()
	defer l.mu.Unlock()
	return int(l.seq)
}

// sthBytes is the canonical byte form signed for a Signed Tree Head.
func sthBytes(cp Checkpoint) []byte {
	// deterministic: exclude Sig; JSON of the fixed struct without it.
	b, _ := json.Marshal(struct {
		NodeID   string `json:"node_id"`
		TreeSize uint64 `json:"tree_size"`
		Root     string `json:"root"`
		PrevCP   string `json:"prev_checkpoint_hash"`
	}{cp.NodeID, cp.TreeSize, cp.Root, cp.PrevCP})
	return b
}
