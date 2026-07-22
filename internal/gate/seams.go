package gate

import "time"

// seams.go declares the distribution seams for the gate's global-correctness stores
// (PLAN-07 R18). A single gate node keeps bans and sticky verdicts in memory, which
// is correct for one process. Across a fleet, though, these two are exactly the state
// that must be SHARED: a ban raised on node A has to block on node B, and a session's
// sticky verdict must follow it to whichever node it lands on. This file names the
// contracts the enforcement path depends on so a production deployment can drop in a
// shared (e.g. Redis) implementation WITHOUT touching Server. No such implementation
// exists yet — these are interfaces only; the compile-time assertions below prove the
// current in-memory stores satisfy them, so the seam can never silently rot.

// BanLedger is the ban store contract the edge enforces against (SoT-27). A shared
// implementation must make Observe's strike accounting atomic across nodes so a
// distributed flood cannot escape the escalation ladder by spreading across the fleet.
type BanLedger interface {
	Check(key string) (BanEntry, bool)
	Observe(key string) (BanEntry, bool, int)
	Add(key, reason, by, incident string, dur time.Duration) BanEntry
	Lift(key string) bool
	List() []BanEntry
}

// VerdictLedger is the sticky-verdict contract (SoT-21 §1). A shared implementation
// makes a verdict assigned on one node immediately visible on the others, so a client
// cannot shed a DENY by reconnecting to a different node behind the load balancer.
type VerdictLedger interface {
	Set(sid string, v stickyVerdict)
	Get(sid string, now time.Time) stickyVerdict
}

// Compile-time proof that the in-memory stores satisfy the seams. If a store's method
// set drifts from the contract, this fails to build — the seam stays honest.
var (
	_ BanLedger     = (*BanStore)(nil)
	_ VerdictLedger = (*VerdictStore)(nil)
)
