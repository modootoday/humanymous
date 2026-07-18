package gate

import (
	"crypto/subtle"
	"net/http"
	"sync"
)

// adminauth.go implements admin-plane authentication + RBAC (SoT-28 WS1/WS2).
// The admin API is served on a SEPARATE listener (see cmd/gate) and every
// request must present a bearer token that maps to an authenticated operator
// with a role. Missing/invalid credentials get 404 (deny-by-default,
// non-discoverable), never 403. The authenticated operator identity is the
// server-derived actor for audit — client-supplied Operator/Approver body
// fields are never trusted as identity.
//
// This is a local reference: operators are seeded with generated bearer tokens
// (production uses mTLS client certs or an SSO/operator-CA login). The RBAC
// choke point and the server-derived-identity property are the load-bearing
// controls and are identical to production.

// Role is an admin RBAC role (SoT-26 §8 / SoT-28 §3).
type Role string

const (
	RoleAuditor  Role = "auditor"  // read-only: view everything, mutate nothing
	RoleOperator Role = "operator" // read + request bans/erasure + lift bans (cannot self-approve)
	RoleApprover Role = "approver" // read + approve pending dual-control actions
	RoleDPO      Role = "dpo"      // read + approve erasure (data-protection officer)
)

// Operator is an authenticated admin principal.
type Operator struct {
	ID   string
	Role Role
}

// AdminAuth maps bearer tokens to operators (constant-time compared).
type AdminAuth struct {
	mu    sync.RWMutex
	byTok map[string]Operator // token -> operator
}

// NewAdminAuth builds an empty auth table.
func NewAdminAuth() *AdminAuth { return &AdminAuth{byTok: map[string]Operator{}} }

// Add registers an operator with a bearer token.
func (a *AdminAuth) Add(token, id string, role Role) {
	a.mu.Lock()
	a.byTok[token] = Operator{ID: id, Role: role}
	a.mu.Unlock()
}

// Authenticate resolves the request's bearer token to an operator. The compare
// is constant-time over all registered tokens so a valid-length guess cannot be
// distinguished by timing.
func (a *AdminAuth) Authenticate(r *http.Request) (Operator, bool) {
	tok := bearer(r)
	if tok == "" {
		return Operator{}, false
	}
	a.mu.RLock()
	defer a.mu.RUnlock()
	// Constant-time scan: compare against every token, never early-exit on match.
	var found Operator
	ok := false
	for t, op := range a.byTok {
		if subtle.ConstantTimeCompare([]byte(t), []byte(tok)) == 1 {
			found, ok = op, true
		}
	}
	return found, ok
}

// bearer extracts the token from Authorization: Bearer <token>.
func bearer(r *http.Request) string {
	h := r.Header.Get("Authorization")
	const p = "Bearer "
	if len(h) > len(p) && h[:len(p)] == p {
		return h[len(p):]
	}
	return ""
}

// capability checks (SoT-28 §3). Auditor is the default least privilege.
func (op Operator) canRead() bool { return op.Role != "" } // any authenticated role reads
func (op Operator) canOperate() bool {
	return op.Role == RoleOperator || op.Role == RoleDPO
}
func (op Operator) canApprove() bool {
	return op.Role == RoleApprover || op.Role == RoleDPO
}
func (op Operator) canApproveErasure() bool { return op.Role == RoleDPO }
