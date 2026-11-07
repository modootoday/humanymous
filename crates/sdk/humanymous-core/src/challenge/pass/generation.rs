use super::{Challenge, ROWS, Row};
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub(super) const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub(super) fn seed(master_key: &[u8], session: &str, bucket: u64, instance: u64) -> [u8; 32] {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(master_key).expect("HMAC accepts keys of any length");
    mac.update(b"pass|");
    mac.update(session.as_bytes());
    mac.update(b"|");
    mac.update(&bucket.to_be_bytes());
    mac.update(b"|");
    mac.update(&instance.to_be_bytes());
    mac.finalize().into_bytes().into()
}

pub(super) struct SplitMix64 {
    pub(super) state: u64,
}
impl SplitMix64 {
    pub(super) fn new(seed: [u8; 32]) -> Self {
        Self {
            state: u64::from_be_bytes(seed[..8].try_into().expect("fixed seed prefix")) | 1,
        }
    }
    pub(super) fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }
    pub(super) fn index(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next() % n as u64) as usize
    }
}

/// Generate a public puzzle from the exact legacy seed and draw order.
///
/// Difficulty is clamped to 0 through 3. No clock, random device, movement or
/// timing requirement is consulted, and a marked cell may already be centered.
pub fn generate(
    master_key: &[u8],
    session: &str,
    bucket: u64,
    instance: u64,
    difficulty: i32,
) -> Challenge {
    let mut random = SplitMix64::new(seed(master_key, session, bucket, instance));
    let n = 9 + difficulty.clamp(0, 3) as usize * 2;
    let mut rows = Vec::with_capacity(ROWS);
    for _ in 0..ROWS {
        let chars = (0..n)
            .map(|_| char::from(ALPHABET[random.index(ALPHABET.len())]).to_string())
            .collect();
        rows.push(Row {
            chars,
            key_index: random.index(n),
        });
    }
    Challenge {
        n,
        center: n / 2,
        rows,
    }
}
