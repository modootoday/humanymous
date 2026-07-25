package gate

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"sync"
	"time"
)

// approval.go implements genuine two-person dual-control (SoT-28 §4). A
// destructive action (permanent/CIDR ban, erasure) is not applied on the first
// request: it creates a PENDING action bound to the requesting operator. A
// DISTINCT authenticated approver must then commit it, and the store enforces
// approver != requester. The committed approval carries an HMAC ticket over the
// exact action parameters so it cannot be replayed onto a different target.
//
// This replaces the previous fiction where "dual-control" was satisfied by any
// non-empty client-supplied Approver string.

// PendingAction is a destructive action awaiting a second approver.
type PendingAction struct {
	ID        string
	Kind      string            // "ban" | "erasure"
	Params    map[string]string // action parameters (key, duration, subject, ...)
	Requester string            // authenticated operator id that requested it
	NeedsRole Role              // role required to approve (Approver or DPO)
	Created   time.Time
}

// ApprovalStore holds pending dual-control actions.
type ApprovalStore struct {
	mu      sync.Mutex
	pending map[string]PendingAction
	key     []byte // HMAC key for approval tickets
	ttl     time.Duration
	nowFn   func() time.Time
}

// NewApprovalStore builds a store with a ticket key and a pending TTL.
func NewApprovalStore(key []byte, ttl time.Duration) *ApprovalStore {
	return &ApprovalStore{pending: map[string]PendingAction{}, key: key, ttl: ttl, nowFn: time.Now}
}

// Create records a pending action and returns its id.
// Params are shallow-copied so a caller cannot retarget a pending dual-control
// action by mutating the map it passed in (wargame r189).
func (s *ApprovalStore) Create(kind string, params map[string]string, requester string, needsRole Role) PendingAction {
	id := randHexN(12)
	cp := make(map[string]string, len(params))
	for k, v := range params {
		cp[k] = v
	}
	p := PendingAction{ID: id, Kind: kind, Params: cp, Requester: requester, NeedsRole: needsRole, Created: s.nowFn()}
	s.mu.Lock()
	s.gcLocked()
	s.pending[id] = p
	s.mu.Unlock()
	return p
}

// Approve commits a pending action by a DISTINCT approver holding the required
// role. It returns the action, a signed ticket, and an error describing any
// rejection (unknown id, expired, self-approval, wrong role).
func (s *ApprovalStore) Approve(id string, approver Operator) (PendingAction, string, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	p, ok := s.pending[id]
	if !ok {
		return PendingAction{}, "", errNoPending
	}
	if s.nowFn().Sub(p.Created) > s.ttl {
		delete(s.pending, id)
		return PendingAction{}, "", errExpired
	}
	if approver.ID == p.Requester {
		return PendingAction{}, "", errSelfApproval
	}
	if !hasRole(approver.Role, p.NeedsRole) {
		return PendingAction{}, "", errWrongRole
	}
	ticket := s.ticket(p, approver.ID)
	delete(s.pending, id)
	return p, ticket, nil
}

// Pending returns a snapshot of pending actions (for the console).
func (s *ApprovalStore) Pending() []PendingAction {
	now := s.nowFn()
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]PendingAction, 0, len(s.pending))
	for id, p := range s.pending {
		if now.Sub(p.Created) > s.ttl {
			delete(s.pending, id)
			continue
		}
		out = append(out, p)
	}
	return out
}

// ticket is an HMAC over {id|kind|requester|approver|params} binding the
// approval to the exact action so it cannot be replayed to a different target.
func (s *ApprovalStore) ticket(p PendingAction, approver string) string {
	m := hmac.New(sha256.New, s.key)
	m.Write([]byte(p.ID + "|" + p.Kind + "|" + p.Requester + "|" + approver))
	// Include settings.overlay binding keys (SoT-39) alongside ban/erasure fields.
	for _, k := range []string{"key", "durationSec", "subject", "legalBasis", "overlayId", "parentConfigVersion", "mutationClass", "body", "action"} {
		m.Write([]byte("|" + k + "=" + p.Params[k]))
	}
	return hex.EncodeToString(m.Sum(nil))
}

func (s *ApprovalStore) gcLocked() {
	now := s.nowFn()
	for id, p := range s.pending {
		if now.Sub(p.Created) > s.ttl {
			delete(s.pending, id)
		}
	}
}

// hasRole reports whether an approver's role satisfies the required role. DPO
// satisfies both Approver and DPO requirements.
func hasRole(have, need Role) bool {
	if have == need {
		return true
	}
	return need == RoleApprover && have == RoleDPO
}

func randHexN(n int) string {
	b := make([]byte, n)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}

// approval errors.
type approvalErr string

func (e approvalErr) Error() string { return string(e) }

const (
	errNoPending    approvalErr = "no such pending action"
	errExpired      approvalErr = "approval expired"
	errSelfApproval approvalErr = "approver must differ from requester (dual-control)"
	errWrongRole    approvalErr = "approver lacks the required role"
)
