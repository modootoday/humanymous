package gate

import (
	"strconv"
	"sync/atomic"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/redis"
)

// redisratelimit.go is the shared, fleet-wide rate counter (PLAN-08 R1 phase 2). The
// per-node abuse.Limiter cannot see a flood SPLIT across gate nodes — each node's
// slice may stay under its local threshold while the aggregate is well over. This
// implementation keeps the count in Redis (an atomic sliding-window sorted set) so
// every node observing a key contributes to ONE rolling window; whichever node
// crosses the hard threshold auto-bans the source, and the ban propagates via the
// RedisBanLedger. On a Redis outage it degrades to per-node local counting (never
// stalls, never blocks) — the same graceful-degradation contract as the ledgers.

// slidingWindowLua atomically evicts entries older than the window, records this
// hit, refreshes the key TTL, and returns the rolling count. KEYS[1]=counter key;
// ARGV: [1]=now_ns, [2]=window_ns, [3]=ttl_ms, [4]=unique member.
const slidingWindowLua = `local now=tonumber(ARGV[1])
local win=tonumber(ARGV[2])
redis.call('ZREMRANGEBYSCORE', KEYS[1], 0, now-win)
redis.call('ZADD', KEYS[1], now, ARGV[4])
redis.call('PEXPIRE', KEYS[1], tonumber(ARGV[3]))
return redis.call('ZCARD', KEYS[1])`

// RedisRateLimiter is a shared sliding-window rate counter satisfying RateLimiter.
type RedisRateLimiter struct {
	rc     *redis.Client
	local  *abuse.Limiter // Level thresholds + outage fallback
	window time.Duration
	seq    uint64 // per-process disambiguator so same-nanosecond hits are distinct members
}

// NewRedisRateLimiter builds a shared limiter with the given window/soft/hard; the
// thresholds match the in-memory limiter so Level classification is identical.
func NewRedisRateLimiter(rc *redis.Client, window time.Duration, soft, hard int) *RedisRateLimiter {
	return &RedisRateLimiter{rc: rc, local: abuse.NewLimiter(window, soft, hard), window: window}
}

// Observe records a hit for key against the SHARED window and returns the aggregate
// rolling count, degrading to the local per-node count if Redis is unavailable.
func (l *RedisRateLimiter) Observe(key string, now time.Time) int {
	if key == "" {
		return 0
	}
	nowNs := now.UnixNano()
	member := strconv.FormatInt(nowNs, 10) + "-" + strconv.FormatUint(atomic.AddUint64(&l.seq, 1), 10)
	rep, err := l.rc.Do("EVAL", slidingWindowLua, "1", "hmn:rl:"+key,
		strconv.FormatInt(nowNs, 10),
		strconv.FormatInt(l.window.Nanoseconds(), 10),
		strconv.FormatInt((l.window*6).Milliseconds(), 10),
		member,
	)
	if err != nil {
		return l.local.Observe(key, now) // outage: fall back to per-node counting
	}
	return int(rep.Int)
}

// Level classifies a rolling count using the shared thresholds.
func (l *RedisRateLimiter) Level(count int) int { return l.local.Level(count) }

// GC sweeps the local fallback limiter (used during a Redis outage). Without it,
// BanStore.GC's type-assertion silently skips this limiter and its per-key map grows
// unbounded during an outage under flood — the reverse of the GC ticker's intent
// (deep-review finding). Redis expires its own ZSETs via PEXPIRE.
func (l *RedisRateLimiter) GC(now time.Time) { l.local.GC(now) }

// Compile-time proof the shared limiter satisfies the seam.
var _ RateLimiter = (*RedisRateLimiter)(nil)
