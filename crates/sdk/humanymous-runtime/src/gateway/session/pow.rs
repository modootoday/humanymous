//! Proof-of-work challenge exchange over the collector store (port of legacy
//! cmd/server/powhandler.go `powSolvedSignal`, `issuePoWHeader` and
//! `handlePoW` at baseline 981d25c; SoT-13/15).
//!
//! A CHALLENGE verdict issues a risk-scaled challenge in `X-HM-PoW`; the
//! difficulty first recorded for the session is re-delivered and verified
//! against, so a mid-challenge risk change never moves the target. A valid
//! submission either reveals a too-fast (native/GPU) solver, which is sticky,
//! or earns the `l7.pow.solved` upgrade; both re-score the session through
//! the caller's engine. The kernels reuse the frozen `proof_of_work` policy,
//! floor and signal registry: nothing here changes a weight, threshold,
//! verdict band or signal identity.
//!
//! Composition stays with the runtime owner: mounting `/api/pow`, extracting
//! the session identity (Go read the `hsid` cookie) and running the scoring
//! engine passed in as `score`.
//!
//! Deliberate differences from Go:
//! - The request body is decoded by a bounded Go-compatible decoder of the
//!   first JSON value (case-folded keys, `null` no-ops, U+FFFD repair of
//!   invalid UTF-8 and lone surrogates, trailing bytes ignored), so the
//!   accept/reject set matches `json.Decoder` over `MaxBytesReader(4096)`.
//! - A stored difficulty without a representable browser floor (64 or more
//!   bits) counts as too fast (fail closed, as in `SolveTime`); Go's shifted
//!   floor was zero. `issue_header` never records more than 17 bits.
//! - A non-finite risk score is not encodable; the reply is the empty JSON
//!   response Go wrote when its encoder failed.
use super::store::CollectorStore;
use crate::gateway::resource::Response;
use humanymous_core::challenge::proof_of_work::{
    WINDOW, difficulty_for, issue, plausible_browser_solve, verify,
};
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use humanymous_core::signals::report::{SessionReport, Timestamp};
use serde::Serialize;

/// Response header carrying an issued challenge.
pub const HEADER: &str = "X-HM-PoW";
/// Largest submission body read (Go `MaxBytesReader(.., 4096)`).
pub const MAX_BODY: usize = 4096;
/// The only verdict that warrants work.
pub const CHALLENGE_VERDICT: &str = "CHALLENGE";
/// Signal granting the PoW trust upgrade.
pub const SOLVED_SIGNAL: &str = "l7.pow.solved";
/// Signal for a solution faster than any browser could compute.
pub const TOO_FAST_SIGNAL: &str = "l7.pow.too_fast";

/// Legacy `powSolvedSignal`.
pub fn solved_signal() -> Signal {
    Signal::new(
        SOLVED_SIGNAL,
        serde_json::Value::Bool(true),
        Verdict::Ok,
        1.0,
        Source::Server,
        "valid PoW solution",
    )
}

fn too_fast_signal(elapsed_ms: i64) -> Signal {
    Signal::new(
        TOO_FAST_SIGNAL,
        serde_json::Value::from(elapsed_ms),
        Verdict::Bot,
        1.0,
        Source::Server,
        "PoW solved faster than a browser could (native/GPU solver)",
    )
}

/// Challenge time bucket of `now` (Go `uint64(now.Unix() / pow.Window)`;
/// pre-epoch times clamp to bucket 0).
pub fn bucket(now: Timestamp) -> u64 {
    let window = i64::try_from(WINDOW).unwrap_or(i64::MAX);
    u64::try_from(now.0.unix_timestamp() / window).unwrap_or(0)
}

/// Legacy `issuePoWHeader`: the `X-HM-PoW` value for a challenged, unsolved
/// session, re-delivering the first-issued difficulty, or `None`. The issue
/// time and difficulty are recorded first-write-wins (unknown sessions are
/// not recorded, but still receive the header, as in Go).
pub fn issue_header(
    store: &mut CollectorStore,
    master_key: &[u8],
    session: &str,
    verdict: &str,
    risk: f64,
    now: Timestamp,
) -> Option<String> {
    if verdict != CHALLENGE_VERDICT || store.pow_solved(session) {
        return None;
    }
    let issued = store.pow_difficulty(session);
    let difficulty = if issued > 0 {
        issued
    } else {
        difficulty_for(risk)
    };
    if difficulty <= 0 {
        return None;
    }
    let header = issue(master_key, session, difficulty, bucket(now)).encode();
    store.set_pow_issued(session, now, difficulty);
    Some(header)
}

/// Decoded `/api/pow` body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Submission {
    /// Challenge bucket the client solved.
    pub bucket: u64,
    /// Client nonce.
    pub nonce: String,
}

/// Decode a submission exactly as Go's `json.NewDecoder(MaxBytesReader(..,
/// 4096)).Decode(&body)` did: only the first JSON value counts, it must end
/// within the first [`MAX_BODY`] bytes, and `None` is Go's `400 bad body`.
/// `body` is what the transport read, at least `MAX_BODY + 1` bytes when
/// available so an over-long body is recognized.
pub fn decode_submission(body: &[u8]) -> Option<Submission> {
    let truncated = body.len() > MAX_BODY;
    let mut parser = Parser {
        input: &body[..body.len().min(MAX_BODY)],
        position: 0,
    };
    parser.whitespace();
    match parser.peek()? {
        b'{' => parser.submission(),
        b'n' => {
            parser.literal(b"null")?;
            // A literal ends only at a following byte or a real end of body.
            (parser.position < parser.input.len() || !truncated).then(Submission::default)
        }
        // Any other top-level value is a syntax or type error in Go.
        _ => None,
    }
}

/// Scoring engine result needed by the reply.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// Session verdict wire label.
    pub verdict: String,
    /// Combined risk score.
    pub risk_score: f64,
}

#[derive(Serialize)]
struct Rejected {
    ok: bool,
}

#[derive(Serialize)]
struct TooFast<'a> {
    ok: bool,
    reason: &'a str,
    verdict: &'a str,
}

#[derive(Serialize)]
struct Solved<'a> {
    ok: bool,
    #[serde(rename = "riskScore")]
    risk_score: f64,
    verdict: &'a str,
}

/// Legacy `handlePoW`. `session` is the request's session identity (empty
/// when absent); `score` re-scores the report in place and returns the
/// engine result, exactly where Go called `engine.Score`.
pub fn handle(
    store: &mut CollectorStore,
    master_key: &[u8],
    method: &str,
    session: &str,
    body: &[u8],
    now: Timestamp,
    score: impl FnOnce(&mut SessionReport) -> Scored,
) -> Response {
    if method != "POST" {
        return Response::text(405, Vec::new(), "POST only");
    }
    if session.is_empty() {
        return Response::text(400, Vec::new(), "no session");
    }
    let Some(submission) = decode_submission(body) else {
        return Response::text(400, Vec::new(), "bad body");
    };
    // Verify at the server-held issue-time difficulty, never one re-derived
    // from the current risk or supplied by the client.
    let difficulty = store.pow_difficulty(session);
    if difficulty <= 0
        || !verify(
            master_key,
            session,
            difficulty,
            submission.bucket,
            bucket(now),
            &submission.nonce,
        )
    {
        return Response::json(&Rejected { ok: false });
    }
    let mut report = store.get(session).unwrap_or_default();
    let issued = store.pow_issued_at(session);
    let elapsed = now.0 - issued.0;
    let floor =
        plausible_browser_solve(difficulty).and_then(|floor| time::Duration::try_from(floor).ok());
    let too_fast_now = issued != Timestamp::default() && floor.is_none_or(|floor| elapsed < floor);
    if too_fast_now || store.pow_too_fast(session) {
        // Permanent: a later plausible resubmit never launders the tell.
        store.set_pow_too_fast(session);
        append(&mut report, too_fast_signal(go_milliseconds(elapsed)));
        let scored = score(&mut report);
        store.store_scored(session, report, now);
        return Response::json(&TooFast {
            ok: false,
            reason: "solve too fast",
            verdict: &scored.verdict,
        });
    }
    store.set_pow_solved(session, now);
    append(&mut report, solved_signal());
    let scored = score(&mut report);
    store.store_scored(session, report, now);
    if !scored.risk_score.is_finite() {
        return encode_failed();
    }
    Response::json(&Solved {
        ok: true,
        risk_score: scored.risk_score,
        verdict: &scored.verdict,
    })
}

fn append(report: &mut SessionReport, signal: Signal) {
    report
        .network
        .signals
        .get_or_insert_with(Vec::new)
        .push(signal);
}

/// Go `time.Duration.Milliseconds` of a saturated `time.Since`.
fn go_milliseconds(elapsed: time::Duration) -> i64 {
    let nanos = elapsed
        .whole_nanoseconds()
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    i64::try_from(nanos / 1_000_000).unwrap_or_default()
}

/// Go `WriteJSON` after an encoder error: the content type only.
fn encode_failed() -> Response {
    Response {
        status: 200,
        headers: vec![("Content-Type", "application/json".to_owned())],
        body: Vec::new(),
    }
}

/// Field of the submission a key selects (Go exact, then case-folded match).
#[derive(Clone, Copy)]
enum Field {
    Bucket,
    Nonce,
    Other,
}

fn field(key: &str) -> Field {
    // Go folds each rune to the smallest of its simple-fold orbit; the only
    // non-ASCII runes that fold into ASCII letters are KELVIN SIGN and LONG S.
    let folded: String = key
        .chars()
        .map(|c| match c {
            '\u{212A}' => 'K',
            '\u{017F}' => 'S',
            c => c.to_ascii_uppercase(),
        })
        .collect();
    match folded.as_str() {
        "BUCKET" => Field::Bucket,
        "NONCE" => Field::Nonce,
        _ => Field::Other,
    }
}

/// Bounded Go `encoding/json` syntax over the decoder's buffered prefix.
struct Parser<'a> {
    input: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        let matched = self.peek() == Some(byte);
        if matched {
            self.position += 1;
        }
        matched
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        self.eat(byte).then_some(())
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn literal(&mut self, text: &[u8]) -> Option<()> {
        let end = self.position.checked_add(text.len())?;
        (self.input.get(self.position..end)? == text).then(|| self.position = end)
    }

    /// Number syntax; returns its text.
    fn number(&mut self) -> Option<&[u8]> {
        let start = self.position;
        self.eat(b'-');
        match self.peek()? {
            b'0' => self.position += 1,
            b'1'..=b'9' => self.digits(),
            _ => return None,
        }
        if self.eat(b'.') {
            self.peek().filter(u8::is_ascii_digit)?;
            self.digits();
        }
        if self.eat(b'e') || self.eat(b'E') {
            let _ = self.eat(b'+') || self.eat(b'-');
            self.peek().filter(u8::is_ascii_digit)?;
            self.digits();
        }
        Some(&self.input[start..self.position])
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
    }

    fn hex4(&self, at: usize) -> Option<u32> {
        let digits = self.input.get(at..at.checked_add(4)?)?;
        digits.iter().try_fold(0u32, |value, byte| {
            char::from(*byte)
                .to_digit(16)
                .map(|digit| (value << 4) | digit)
        })
    }

    /// String syntax and Go unquoting (invalid UTF-8 and unpaired surrogates
    /// become U+FFFD, one per invalid byte or escape).
    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let byte = self.peek()?;
            match byte {
                b'"' => {
                    self.position += 1;
                    return Some(out);
                }
                0..=0x1f => return None,
                b'\\' => {
                    self.position += 1;
                    let escape = self.peek()?;
                    self.position += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => return None,
                    }
                }
                0x20..=0x7f => {
                    out.push(char::from(byte));
                    self.position += 1;
                }
                _ => {
                    // Go `utf8.DecodeRune`: one valid rune, else U+FFFD for
                    // this single byte.
                    let width = match byte {
                        0xC2..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        0xF0..=0xF4 => 4,
                        _ => 0,
                    };
                    let rune = self
                        .input
                        .get(self.position..self.position + width)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok());
                    match rune {
                        Some(text) if width > 0 => {
                            out.push_str(text);
                            self.position += width;
                        }
                        _ => {
                            out.push(char::REPLACEMENT_CHARACTER);
                            self.position += 1;
                        }
                    }
                }
            }
        }
    }

    /// Go `getu4` plus `utf16.DecodeRune`, after the `\u` was consumed.
    fn unicode_escape(&mut self) -> Option<char> {
        let first = self.hex4(self.position)?;
        self.position += 4;
        if !(0xD800..0xE000).contains(&first) {
            return char::from_u32(first);
        }
        let next = (self.input.get(self.position..self.position + 2) == Some(b"\\u"))
            .then(|| self.hex4(self.position + 2))
            .flatten();
        match next {
            Some(low) if (0xD800..0xDC00).contains(&first) && (0xDC00..0xE000).contains(&low) => {
                self.position += 6;
                char::from_u32(0x10000 + ((first - 0xD800) << 10) + (low - 0xDC00))
            }
            _ => Some(char::REPLACEMENT_CHARACTER),
        }
    }

    /// Validate and skip one value of any shape without recursion.
    fn skip_value(&mut self) -> Option<()> {
        let mut closers: Vec<u8> = Vec::new();
        loop {
            self.whitespace();
            match self.peek()? {
                b'{' => {
                    self.position += 1;
                    self.whitespace();
                    if !self.eat(b'}') {
                        self.member_key()?;
                        closers.push(b'}');
                        continue;
                    }
                }
                b'[' => {
                    self.position += 1;
                    self.whitespace();
                    if !self.eat(b']') {
                        closers.push(b']');
                        continue;
                    }
                }
                b'"' => {
                    self.string()?;
                }
                b't' => self.literal(b"true")?,
                b'f' => self.literal(b"false")?,
                b'n' => self.literal(b"null")?,
                b'-' | b'0'..=b'9' => {
                    self.number()?;
                }
                _ => return None,
            }
            loop {
                let Some(&closer) = closers.last() else {
                    return Some(());
                };
                self.whitespace();
                if self.eat(b',') {
                    if closer == b'}' {
                        self.whitespace();
                        self.member_key()?;
                    }
                    break;
                }
                self.expect(closer)?;
                closers.pop();
            }
        }
    }

    /// `"key"` then `:`; returns the unquoted key.
    fn member_key(&mut self) -> Option<String> {
        let key = self.string()?;
        self.whitespace();
        self.expect(b':')?;
        Some(key)
    }

    /// The top-level object into the submission fields. A type mismatch is
    /// remembered and fails the decode at the end, as Go's saved error did.
    fn submission(&mut self) -> Option<Submission> {
        let mut submission = Submission::default();
        let mut type_error = false;
        self.expect(b'{')?;
        self.whitespace();
        if !self.eat(b'}') {
            loop {
                self.whitespace();
                let key = self.member_key()?;
                self.whitespace();
                match (field(&key), self.peek()?) {
                    (Field::Bucket | Field::Nonce, b'n') => self.literal(b"null")?,
                    (Field::Bucket, b'-' | b'0'..=b'9') => {
                        let text = self.number()?;
                        // Go ParseUint: unsigned decimal digits within u64.
                        match std::str::from_utf8(text).ok().and_then(|text| {
                            text.bytes()
                                .all(|byte| byte.is_ascii_digit())
                                .then(|| text.parse::<u64>().ok())
                                .flatten()
                        }) {
                            Some(value) => submission.bucket = value,
                            None => type_error = true,
                        }
                    }
                    (Field::Nonce, b'"') => submission.nonce = self.string()?,
                    (Field::Bucket | Field::Nonce, _) => {
                        self.skip_value()?;
                        type_error = true;
                    }
                    (Field::Other, _) => self.skip_value()?,
                }
                self.whitespace();
                if self.eat(b'}') {
                    break;
                }
                self.expect(b',')?;
            }
        }
        (!type_error).then_some(submission)
    }
}

#[cfg(test)]
mod tests;
