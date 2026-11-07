use super::{PublicRoleDataError, wire};

/// Original-document binding assertions, not a generation claim or native proof.
pub struct RoleBinding {
    epoch: [u8; 16],
    generation: u64,
    base_plan: [u8; 32],
    executable: [u8; 32],
}
impl RoleBinding {
    pub(super) fn from_wire(value: wire::Binding) -> Result<Self, PublicRoleDataError> {
        let epoch = decode(&value.installation_epoch)?;
        if epoch == [0; 16] || !(1..=9_007_199_254_740_991).contains(&value.generation) {
            return Err(PublicRoleDataError::Invalid);
        }
        Ok(Self {
            epoch,
            generation: value.generation,
            base_plan: decode(&value.base_plan_sha256)?,
            executable: decode(&value.executable_sha256)?,
        })
    }
    /// Nonzero declared epoch; its bytes do not prove entropy or freshness.
    pub fn installation_epoch(&self) -> &[u8; 16] {
        &self.epoch
    }
    /// Positive bounded declared generation, not a consumed replay slot.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Digest of an external original BASE document, never this envelope.
    pub fn base_plan_sha256(&self) -> &[u8; 32] {
        &self.base_plan
    }
    /// Declared artifact digest, not verified executable custody.
    pub fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable
    }
}
fn decode<const N: usize>(value: &str) -> Result<[u8; N], PublicRoleDataError> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(PublicRoleDataError::Invalid);
    }
    let mut bytes = [0; N];
    hex::decode_to_slice(value, &mut bytes).map_err(|_| PublicRoleDataError::Invalid)?;
    Ok(bytes)
}
