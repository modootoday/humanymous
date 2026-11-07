use super::{MAX_BUCKET_AGE, ROWS, generate};

/// Accept only nonfuture issuance no more than twenty supplied buckets old.
pub fn fresh(issued: u64, current: u64) -> bool {
    current >= issued && current - issued <= MAX_BUCKET_AGE
}

/// Verify only public alignment and freshness using the 64-bit legacy profile.
///
/// Signed offsets deliberately wrap on key-index addition before normalization,
/// matching 64-bit Go arithmetic even at i64 extremes. This is not authentication
/// or single-use enforcement, and different instances may have the same answer.
pub fn verify(
    master_key: &[u8],
    session: &str,
    bucket: u64,
    current: u64,
    instance: u64,
    difficulty: i32,
    offsets: &[i64],
) -> bool {
    if !fresh(bucket, current) || offsets.len() != ROWS {
        return false;
    }
    let challenge = generate(master_key, session, bucket, instance, difficulty);
    challenge.rows.iter().zip(offsets).all(|(row, offset)| {
        (row.key_index as i64)
            .wrapping_add(*offset)
            .rem_euclid(challenge.n as i64)
            == challenge.center as i64
    })
}

/// Derive the public canonical nonnegative shifts; these are not secret answers.
pub fn solution_offsets(
    master_key: &[u8],
    session: &str,
    bucket: u64,
    instance: u64,
    difficulty: i32,
) -> Vec<i64> {
    let challenge = generate(master_key, session, bucket, instance, difficulty);
    challenge
        .rows
        .iter()
        .map(|row| (challenge.center as i64 - row.key_index as i64).rem_euclid(challenge.n as i64))
        .collect()
}
