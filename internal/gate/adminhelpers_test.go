package gate

import (
	"net/http/httptest"
	"strings"
	"testing"
)

// seedAdmins registers the four RBAC roles with deterministic dev tokens and
// returns role→token (SoT-28 WS1/WS2 test support).
func seedAdmins(_ *testing.T, srv *Server) map[Role]string {
	toks := map[Role]string{}
	for _, role := range []Role{RoleAuditor, RoleOperator, RoleApprover, RoleDPO} {
		tk := string(role) + "-token-0123456789"
		toks[role] = tk
		srv.Auth().Add(tk, string(role)+"-1", role)
	}
	return toks
}

// adminDo issues an admin-plane request through the SEPARATE AdminHandler with an
// optional bearer token (empty = unauthenticated).
func adminDo(srv *Server, token, method, path, body string) *httptest.ResponseRecorder {
	r := httptest.NewRequest(method, "http://admin"+path, strings.NewReader(body))
	if token != "" {
		r.Header.Set("Authorization", "Bearer "+token)
	}
	if body != "" {
		r.Header.Set("Content-Type", "application/json")
	}
	w := httptest.NewRecorder()
	srv.AdminHandler().ServeHTTP(w, r)
	return w
}
