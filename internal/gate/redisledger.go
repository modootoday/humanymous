package gate

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"strconv"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/redis"
)

// sealValue appends an HMAC-SHA256 tag over (ctx || 0x00 || payload) so a compromised
// coordinator that can write Redis cannot FORGE a value NOR RELOCATE an authentic one:
// the tag is bound to ctx (the record's Redis KEY), so a value sealed for one key does
// not verify under another (defeats verdict/ban relocation). Freshness (rollback) is
// enforced by the caller re-checking the payload's own timestamp on read. With no key
// configured it is a no-op (unsigned, backward-compatible). NUL separator: JSON never
// emits a NUL byte.
func sealValue(key []byte, ctx, payload string) string {
	if len(key) == 0 {
		return payload
	}
	return payload + "\x00" + hex.EncodeToString(sealTag(key, ctx, payload))
}

// openValue verifies the key-bound HMAC tag and strips it. In signed mode an unsigned,
// relocated, or tag-mismatched value is rejected; the caller falls back to its trusted
// local view rather than honoring forged/relocated coordinator state.
func openValue(key []byte, ctx, val string) (string, bool) {
	if len(key) == 0 {
		return val, true
	}
	i := strings.LastIndexByte(val, '\x00')
	if i < 0 {
		return "", false
	}
	payload, tag := val[:i], val[i+1:]
	if !hmac.Equal([]byte(tag), []byte(hex.EncodeToString(sealTag(key, ctx, payload)))) {
		return "", false
	}
	return payload, true
}

func sealTag(key []byte, ctx, payload string) []byte {
	m := hmac.New(sha256.New, key)
	m.Write([]byte(ctx))
	m.Write([]byte{0})
	m.Write([]byte(payload))
	return m.Sum(nil)
}

// redisledger.go provides Redis-backed implementations of the R18 distribution
// seams (BanLedger, VerdictLedger) so bans and sticky verdicts are shared across a
// gate fleet (PLAN-08 R1): a DENY or a ban raised on one node is enforced on the
// others, closing the "reconnect to a fresh node to shed the verdict" gap.
//
// Each ledger wraps an in-memory mirror used BOTH as a write-through cache and as an
// outage fallback. When Redis is reachable, state propagates fleet-wide; when it is
// not, the node degrades to its local view rather than failing the whole edge. This
// graceful degradation (serve-local over deny-everything) is deliberate: it matches
// the project's no-lockout ethos — a coordination outage must never take the site
// down or wedge the wargame. Keys are namespaced hmn:{ban,verdict}:<k>.

// keyspace prefixes.
const (
	redisBanPrefix     = "hmn:ban:"
	redisVerdictPrefix = "hmn:verdict:"
)

// --- VerdictLedger ---------------------------------------------------------

// RedisVerdictLedger shares sticky verdicts across nodes. Set writes through to
// Redis (with a PX expiry matching the local TTL) and mirrors locally; Get reads
// Redis first so a verdict assigned on another node is visible, falling back to the
// local mirror on a Redis outage.
type RedisVerdictLedger struct {
	rc    *redis.Client
	local *VerdictStore
	ttl   time.Duration
	sign  []byte // fleet-shared HMAC key; nil = unsigned (PLAN-08 backlog)
}

// verdictWire is the JSON form of a stickyVerdict (whose fields are unexported).
type verdictWire struct {
	Verdict Verdict        `json:"v"`
	Risk    float64        `json:"r"`
	Rule    string         `json:"h"`
	Top     []audit.Signal `json:"t"`
	Updated time.Time      `json:"u"`
}

// NewRedisVerdictLedger builds a shared verdict ledger over rc with the given TTL.
// signKey (fleet-shared) HMAC-authenticates stored values; nil = unsigned.
func NewRedisVerdictLedger(rc *redis.Client, ttl time.Duration, signKey []byte) *RedisVerdictLedger {
	return &RedisVerdictLedger{rc: rc, local: NewVerdictStore(ttl), ttl: ttl, sign: signKey}
}

func (l *RedisVerdictLedger) Set(sid string, v stickyVerdict) {
	l.local.Set(sid, v) // mirror for outage fallback
	b, err := json.Marshal(verdictWire{v.verdict, v.risk, v.rule, v.top, v.updated})
	if err != nil {
		return
	}
	_, _ = l.rc.Do("SET", redisVerdictPrefix+sid, sealValue(l.sign, redisVerdictPrefix+sid, string(b)), "PX", strconv.FormatInt(l.ttl.Milliseconds(), 10))
}

// GC sweeps the local mirror (Redis expires its own keys via PX).
func (l *RedisVerdictLedger) GC(now time.Time) { l.local.GC(now) }

func (l *RedisVerdictLedger) Get(sid string, now time.Time) stickyVerdict {
	rep, err := l.rc.Do("GET", redisVerdictPrefix+sid)
	if err != nil {
		return l.local.Get(sid, now) // outage: serve the local view
	}
	if rep.Nil {
		return stickyVerdict{verdict: VerdictUnknown} // no shared verdict yet
	}
	payload, ok := openValue(l.sign, redisVerdictPrefix+sid, rep.Str)
	if !ok {
		return l.local.Get(sid, now) // forged/relocated coordinator value → trust local
	}
	var w verdictWire
	if json.Unmarshal([]byte(payload), &w) != nil {
		return l.local.Get(sid, now)
	}
	if now.Sub(w.Updated) > l.ttl { // freshness: reject a replayed/rolled-back stale verdict
		return l.local.Get(sid, now)
	}
	return stickyVerdict{verdict: w.Verdict, risk: w.Risk, rule: w.Rule, top: w.Top, updated: w.Updated}
}

// --- BanLedger -------------------------------------------------------------

// RedisBanLedger shares bans across nodes. Check reads Redis (authoritative) so a
// ban applied anywhere is enforced everywhere; Add/Observe/Lift write through to
// Redis. The escalating auto-ban ladder + rate-limit breach detection still run in
// the local mirror (per-node breach detection); PLAN-08 phase 2 adds shared GCRA so
// a flood split across nodes escalates on aggregate. Reads fall back to local on a
// Redis outage.
type RedisBanLedger struct {
	rc    *redis.Client
	local *BanStore
	nowFn func() time.Time
	sign  []byte // fleet-shared HMAC key; nil = unsigned (PLAN-08 backlog)
}

// banWire is the JSON form of a BanEntry (all fields exported → direct marshal).
type banWire = BanEntry

// NewRedisBanLedger builds a shared ban ledger. The auto-ban rate detection runs on
// a SHARED sliding-window counter (RedisRateLimiter) so a flood split across nodes
// escalates on the aggregate (PLAN-08 R1 phase 2); the ban itself + strike ladder
// live in the inner BanStore and propagate via writeBan. Both degrade to per-node
// local behavior on a Redis outage.
func NewRedisBanLedger(rc *redis.Client, window time.Duration, soft, hard int, signKey []byte) *RedisBanLedger {
	rl := NewRedisRateLimiter(rc, window, soft, hard)
	return &RedisBanLedger{rc: rc, local: NewBanStoreWithLimiter(rl), nowFn: time.Now, sign: signKey}
}

// GC sweeps the local mirror + its rate-limiter (Redis expires its own keys).
func (l *RedisBanLedger) GC(now time.Time) { l.local.GC(now) }

func (l *RedisBanLedger) Check(key string) (BanEntry, bool) {
	rep, err := l.rc.Do("GET", redisBanPrefix+key)
	if err != nil {
		return l.local.Check(key) // outage fallback
	}
	if rep.Nil {
		return BanEntry{}, false
	}
	payload, ok := openValue(l.sign, redisBanPrefix+key, rep.Str)
	if !ok {
		return l.local.Check(key) // forged/relocated ban → trust local (ignore injection)
	}
	var b banWire
	if json.Unmarshal([]byte(payload), &b) != nil || b.Key != key {
		return l.local.Check(key) // value not sealed for THIS key
	}
	if !b.active(l.nowFn()) { // TTL should have removed it, but honor Until defensively
		return BanEntry{}, false
	}
	return b, true
}

func (l *RedisBanLedger) Observe(key string) (BanEntry, bool, int) {
	entry, banned, level := l.local.Observe(key)
	if banned {
		l.writeBan(key, entry) // propagate the auto-ban fleet-wide
	}
	return entry, banned, level
}

func (l *RedisBanLedger) Add(key, reason, by, incident string, dur time.Duration) BanEntry {
	entry := l.local.Add(key, reason, by, incident, dur) // reuse the ban construction
	l.writeBan(key, entry)
	return entry
}

func (l *RedisBanLedger) Lift(key string) bool {
	lifted := l.local.Lift(key)
	_, _ = l.rc.Do("DEL", redisBanPrefix+key)
	return lifted
}

// List returns the union of the fleet-wide bans in Redis and the local mirror,
// deduped by key (Redis wins). On a Redis outage it returns the local view.
func (l *RedisBanLedger) List() []BanEntry {
	now := l.nowFn()
	seen := map[string]bool{}
	out := []BanEntry{}
	for _, key := range l.scanBanKeys() {
		rep, err := l.rc.Do("GET", key)
		if err != nil {
			return l.local.List() // outage mid-scan: fall back to a consistent local view
		}
		if rep.Nil {
			continue
		}
		payload, ok := openValue(l.sign, key, rep.Str)
		if !ok {
			continue // ignore forged/relocated entries
		}
		var b banWire
		if json.Unmarshal([]byte(payload), &b) != nil || !b.active(now) || redisBanPrefix+b.Key != key {
			continue
		}
		seen[b.Key] = true
		out = append(out, b)
	}
	for _, b := range l.local.List() { // include local-only entries (e.g. pre-outage)
		if !seen[b.Key] {
			out = append(out, b)
		}
	}
	return out
}

// writeBan mirrors a ban to Redis with a PX expiry equal to its remaining lifetime
// (permanent bans are stored without expiry).
func (l *RedisBanLedger) writeBan(key string, entry BanEntry) {
	b, err := json.Marshal(entry)
	if err != nil {
		return
	}
	val := sealValue(l.sign, redisBanPrefix+key, string(b))
	if entry.Permanent() {
		_, _ = l.rc.Do("SET", redisBanPrefix+key, val)
		return
	}
	ms := time.Until(entry.Until).Milliseconds()
	if ms <= 0 {
		return
	}
	_, _ = l.rc.Do("SET", redisBanPrefix+key, val, "PX", strconv.FormatInt(ms, 10))
}

// scanBan* bounds so a hostile/buggy coordinator that never returns cursor "0" (or keeps
// emitting fresh keys) cannot spin scanBanKeys forever — which would peg CPU, grow the
// slice until OOM, and, because every Do() takes the single shared-connection mutex,
// starve every concurrent on-path ban/verdict/rate lookup fleet-wide (deep-review). Each
// SCAN round-trip is fast enough to slip under the client's slow-breaker, so the loop must
// self-limit. The caps are far above any real ban keyspace; hitting one returns the partial
// set (List still merges the local mirror) rather than serving nothing.
const (
	scanMaxRounds = 4096    // max SCAN round-trips per enumeration
	scanMaxKeys   = 1 << 16 // 65,536 ban keys is already far past any real deployment
)

// scanBanKeys enumerates ban keys via non-blocking SCAN (MATCH hmn:ban:*), bounded.
func (l *RedisBanLedger) scanBanKeys() []string {
	var keys []string
	cursor := "0"
	seen := map[string]bool{} // detect a non-advancing / cycling cursor
	for round := 0; round < scanMaxRounds; round++ {
		rep, err := l.rc.Do("SCAN", cursor, "MATCH", redisBanPrefix+"*", "COUNT", "256")
		if err != nil || len(rep.Array) != 2 {
			return keys
		}
		cursor = rep.Array[0].Str
		for _, k := range rep.Array[1].Array {
			keys = append(keys, k.Str)
			if len(keys) >= scanMaxKeys {
				return keys // key cap: stop growing (partial set)
			}
		}
		if cursor == "0" {
			return keys // normal completion
		}
		if seen[cursor] { // coordinator is replaying a cursor → never converges
			return keys
		}
		seen[cursor] = true
	}
	return keys // round cap: coordinator never returned "0"
}

// Compile-time proof the Redis ledgers satisfy the R18 distribution seams.
var (
	_ BanLedger     = (*RedisBanLedger)(nil)
	_ VerdictLedger = (*RedisVerdictLedger)(nil)
)
