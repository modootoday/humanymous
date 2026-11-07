use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u8,
    pub candidate_id: String,
    pub repository_root: PathBuf,
    pub materialization_path: PathBuf,
    pub retained_root: PathBuf,
    pub scratch_path: PathBuf,
    pub owner: Owner,
    pub source: Source,
    pub consumers: Consumers,
    pub archive: Archive,
    pub coverage: Coverage,
    pub members: Vec<Member>,
    pub readback: Readback,
    pub retention: Retention,
    pub proposed_action: Action,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub lane: String,
    pub provider: String,
    pub session: String,
    pub disposition_authority_receipt: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub checkpoint: String,
    pub immutable_identity_sha256: String,
    pub historical_executed_path: PathBuf,
    pub original_receipt: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumers {
    pub handles: Vec<Handle>,
    pub inventory_complete_asserted: bool,
    pub no_remaining_consumers_asserted: bool,
    pub released_lease_receipts: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handle {
    pub id: String,
    pub kind: HandleKind,
    pub state: HandleState,
    pub exit_code: Option<i32>,
    pub terminal_receipt: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleKind {
    Producer,
    Reader,
    BuildTest,
    NestedSession,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleState {
    Terminal,
    Active,
    Unknown,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub manifest_path: PathBuf,
    pub format: ArchiveFormat,
    pub extraction_profile_receipt: String,
    pub whole_sha256: String,
    pub whole_bytes: u64,
    pub creation_receipt: String,
    pub parts: Vec<Part>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    TarZstd,
    Zip,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub ordinal: usize,
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub member_count: usize,
    pub regular_file_bytes: u64,
    pub complete_asserted: bool,
    pub traversal_policy: String,
    pub case_collision_policy: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub path: String,
    pub kind: MemberKind,
    pub bytes: u64,
    pub sha256: String,
    pub link_target: String,
    pub mode: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    File,
    Directory,
    Link,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Readback {
    pub receipt: String,
    pub member_manifest_sha256: String,
    pub archive_sha256: String,
    pub complete_asserted: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    pub private_access_policy_receipt: String,
    pub dependency_receipts: Vec<String>,
    pub retention_policy_receipt: String,
    pub rollback_receipt: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    RetireMaterialization,
}

#[derive(Debug, Serialize)]
pub struct Plan {
    pub schema: &'static str,
    pub state: &'static str,
    pub structurally_valid: bool,
    pub supplied_assertions_only: bool,
    pub candidate_filesystem_verified: bool,
    pub terminal_handles_independently_verified: bool,
    pub archive_bytes_verified: bool,
    pub readback_independently_verified: bool,
    pub owner_authority_verified: bool,
    pub retirement_authorized: bool,
    pub execution_authorized: bool,
    pub proposed_member_count: usize,
    pub proposed_part_count: usize,
    pub proposed_member_bytes: u64,
    pub proposed_archive_bytes: u64,
    pub proposed_action: &'static str,
    pub proposed_archive_format: &'static str,
    /// Producer, reader, build/test, nested-session assertions, respectively.
    pub proposed_handle_counts: [usize; 4],
    pub member_manifest_sha256: String,
    pub errors: Vec<&'static str>,
    pub required_next_gates: [&'static str; 5],
}
