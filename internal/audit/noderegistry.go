package audit

import (
	"crypto/ed25519"
	"sync"
	"time"
)

// noderegistry.go detects node SUPPRESSION (SoT-28 WS8). The strongest attack on
// a per-node audit chain is not to tamper with a node but to STOP it: with no
// registry of expected nodes and no liveness signal, a killed node's absence
// produces no alert and its whole attack window vanishes. The registry holds the
// expected node set (id + pubkey + heartbeat cadence); each node emits a signed
// heartbeat at its cadence even when idle, and a monitor flags any registered
// node that misses its window as suppression-suspected.
//
// A node cannot detect its OWN suppression in-process; the registry is held by a
// monitor / peer node (or the witness process). The mechanism and its detection
// are what this provides; production runs the monitor out of process.

// NodeInfo is an expected node's registration + last-seen liveness.
type NodeInfo struct {
	NodeID   string
	Interval time.Duration
	PubKey   ed25519.PublicKey
	Last     time.Time
}

// NodeRegistry tracks expected nodes and their heartbeats.
type NodeRegistry struct {
	mu    sync.Mutex
	nodes map[string]*NodeInfo
	grace float64 // a node is "missing" after grace × Interval with no heartbeat
}

// NewNodeRegistry builds a registry (missing after 2× the cadence by default).
func NewNodeRegistry() *NodeRegistry {
	return &NodeRegistry{nodes: map[string]*NodeInfo{}, grace: 2}
}

// Expect registers a node that MUST heartbeat every interval.
func (r *NodeRegistry) Expect(nodeID string, interval time.Duration, pub ed25519.PublicKey, now time.Time) {
	r.mu.Lock()
	r.nodes[nodeID] = &NodeInfo{NodeID: nodeID, Interval: interval, PubKey: pub, Last: now}
	r.mu.Unlock()
}

// Heartbeat records liveness for a node (called when a signed heartbeat arrives).
func (r *NodeRegistry) Heartbeat(nodeID string, now time.Time) {
	r.mu.Lock()
	if n, ok := r.nodes[nodeID]; ok {
		n.Last = now
	}
	r.mu.Unlock()
}

// Missing returns the ids of registered nodes that have not heartbeat within
// grace × their interval — i.e. suppression-suspected (SoT-28 WS8).
func (r *NodeRegistry) Missing(now time.Time) []string {
	r.mu.Lock()
	defer r.mu.Unlock()
	var out []string
	for id, n := range r.nodes {
		if now.Sub(n.Last) > time.Duration(r.grace*float64(n.Interval)) {
			out = append(out, id)
		}
	}
	return out
}

// Nodes returns a snapshot of the expected set (for the console).
func (r *NodeRegistry) Nodes(now time.Time) []map[string]any {
	r.mu.Lock()
	defer r.mu.Unlock()
	out := make([]map[string]any, 0, len(r.nodes))
	for _, n := range r.nodes {
		out = append(out, map[string]any{
			"node":       n.NodeID,
			"lastSeenMs": now.Sub(n.Last).Milliseconds(),
			"missing":    now.Sub(n.Last) > time.Duration(r.grace*float64(n.Interval)),
		})
	}
	return out
}
