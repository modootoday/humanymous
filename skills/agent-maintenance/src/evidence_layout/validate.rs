use super::model::*;
use crate::selected_inputs::Limits;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

fn text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 512
        && !value.chars().any(char::is_control)
        && !["unknown", "refused", "missing", "none", "not_run"]
            .contains(&value.trim().to_ascii_lowercase().as_str())
}
fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.contains(['\\', ':'])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != ".." && !s.ends_with(['.', ' ']))
}
fn absolute(path: &Path) -> bool {
    let Some(value) = path.to_str() else {
        return false;
    };
    if value.len() > 4096
        || value.chars().any(char::is_control)
        || !path.is_absolute()
        || value
            .replace('\\', "/")
            .split('/')
            .any(|p| p == "." || p == "..")
    {
        return false;
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_)))
    {
        return false;
    }
    path.components().any(|c| matches!(c, Component::Normal(_)))
        && !path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
}
fn key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}
fn within(path: &Path, root: &Path) -> bool {
    key(path).starts_with(&(key(root) + "/"))
}
fn need(errors: &mut BTreeSet<&'static str>, accepted: bool, code: &'static str) {
    if !accepted {
        errors.insert(code);
    }
}

/// Fingerprint supplied member metadata, never observed source bytes or coverage.
pub(super) fn member_digest(members: &[Member]) -> String {
    let mut ordered: Vec<_> = members.iter().collect();
    ordered.sort_by(|a, b| a.path.cmp(&b.path));
    let mut digest = Sha256::new();
    digest.update(b"humanymous.supplied-members.v1\0");
    for member in ordered {
        for value in [&member.path, &member.sha256, &member.link_target] {
            digest.update((value.len() as u64).to_le_bytes());
            digest.update(value.as_bytes());
        }
        digest.update([match member.kind {
            MemberKind::File => 0,
            MemberKind::Directory => 1,
            MemberKind::Link => 2,
        }]);
        digest.update(member.bytes.to_le_bytes());
        digest.update(member.mode.to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub fn plan(request: &Request) -> Plan {
    let limits = Limits::default();
    let mut errors = BTreeSet::new();
    need(
        &mut errors,
        request.schema_version == 1,
        "unsupported_version",
    );
    need(&mut errors, text(&request.candidate_id), "candidate_id");
    need(
        &mut errors,
        [
            &request.owner.lane,
            &request.owner.provider,
            &request.owner.session,
            &request.owner.disposition_authority_receipt,
        ]
        .iter()
        .all(|s| text(s)),
        "owner_assertion",
    );
    need(
        &mut errors,
        ["codex", "claude", "grok", "gemini", "cursor", "human"].contains(&request.owner.provider.as_str()),
        "owner_provider",
    );
    let paths = [
        &request.repository_root,
        &request.materialization_path,
        &request.retained_root,
        &request.scratch_path,
        &request.source.historical_executed_path,
        &request.archive.manifest_path,
    ];
    need(
        &mut errors,
        paths.iter().all(|p| absolute(p)),
        "absolute_paths",
    );
    let canonical_root = request.repository_root.join(".spec.local");
    need(
        &mut errors,
        within(&request.retained_root, &canonical_root),
        "private_retention_root",
    );
    need(
        &mut errors,
        within(&request.archive.manifest_path, &request.retained_root),
        "archive_manifest_location",
    );
    need(
        &mut errors,
        !within(&request.scratch_path, &canonical_root)
            && key(&request.scratch_path) != key(&canonical_root),
        "scratch_inside_canonical_tree",
    );
    need(
        &mut errors,
        key(&request.scratch_path) != key(&request.repository_root)
            && !within(&request.repository_root, &request.scratch_path)
            && !within(&request.materialization_path, &request.scratch_path),
        "scratch_protected_overlap",
    );
    need(
        &mut errors,
        !within(&request.retained_root, &request.materialization_path)
            && !within(&request.scratch_path, &request.materialization_path)
            && key(&request.materialization_path) != key(&request.retained_root)
            && key(&request.materialization_path) != key(&request.scratch_path)
            && key(&request.materialization_path) != key(&request.repository_root)
            && !within(&request.repository_root, &request.materialization_path),
        "protected_path_overlap",
    );
    need(
        &mut errors,
        [40, 64].contains(&request.source.checkpoint.len())
            && request
                .source
                .checkpoint
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            && hash(&request.source.immutable_identity_sha256)
            && text(&request.source.original_receipt),
        "source_identity_assertion",
    );
    let consumers = &request.consumers;
    need(
        &mut errors,
        consumers.inventory_complete_asserted && consumers.no_remaining_consumers_asserted,
        "consumer_coverage_unknown",
    );
    need(
        &mut errors,
        !consumers.handles.is_empty() && consumers.handles.len() <= limits.max_roots,
        "handle_count",
    );
    need(
        &mut errors,
        !consumers.released_lease_receipts.is_empty()
            && consumers.released_lease_receipts.len() <= limits.max_roots
            && consumers.released_lease_receipts.iter().all(|s| text(s)),
        "lease_assertions",
    );
    let mut handles = BTreeSet::new();
    let mut proposed_handle_counts = [0_usize; 4];
    for handle in &consumers.handles {
        need(
            &mut errors,
            text(&handle.id)
                && handles.insert(&handle.id)
                && matches!(handle.state, HandleState::Terminal)
                && handle.exit_code.is_some()
                && text(&handle.terminal_receipt),
            "terminal_handle_assertion",
        );
        proposed_handle_counts[match handle.kind {
            HandleKind::Producer => 0,
            HandleKind::Reader => 1,
            HandleKind::BuildTest => 2,
            HandleKind::NestedSession => 3,
        }] += 1;
    }
    let archive = &request.archive;
    let proposed_archive_format = match archive.format {
        ArchiveFormat::TarZstd => "tar_zstd",
        ArchiveFormat::Zip => "zip",
    };
    need(
        &mut errors,
        text(&archive.extraction_profile_receipt)
            && text(&archive.creation_receipt)
            && hash(&archive.whole_sha256),
        "archive_assertion",
    );
    need(
        &mut errors,
        !archive.parts.is_empty() && archive.parts.len() <= limits.max_roots,
        "archive_part_count",
    );
    let mut part_paths = BTreeSet::new();
    let mut part_total = 0_u64;
    for (index, part) in archive.parts.iter().enumerate() {
        need(
            &mut errors,
            part.ordinal == index
                && absolute(&part.path)
                && within(&part.path, &request.retained_root)
                && key(&part.path) != key(&archive.manifest_path)
                && part_paths.insert(key(&part.path))
                && part.bytes > 0
                && part.bytes <= limits.max_file_bytes
                && hash(&part.sha256),
            "archive_part_assertion",
        );
        part_total = match part_total.checked_add(part.bytes) {
            Some(n) => n,
            None => {
                errors.insert("byte_overflow");
                u64::MAX
            }
        };
    }
    need(
        &mut errors,
        part_total == archive.whole_bytes && part_total <= limits.max_total_read_bytes,
        "archive_byte_coverage",
    );
    if archive.parts.len() == 1 {
        need(
            &mut errors,
            archive.parts[0].sha256 == archive.whole_sha256,
            "single_part_identity",
        );
    }
    need(
        &mut errors,
        !request.members.is_empty() && request.members.len() <= limits.max_files,
        "member_count",
    );
    let mut members = BTreeMap::new();
    let mut member_total = 0_u64;
    for member in &request.members {
        need(
            &mut errors,
            relative(&member.path)
                && member.mode <= 0o7777
                && members
                    .insert(member.path.to_lowercase(), &member.kind)
                    .is_none(),
            "member_path_or_case_collision",
        );
        let valid = match member.kind {
            MemberKind::File => {
                member_total = match member_total.checked_add(member.bytes) {
                    Some(n) => n,
                    None => {
                        errors.insert("byte_overflow");
                        u64::MAX
                    }
                };
                hash(&member.sha256)
                    && member.link_target.is_empty()
                    && member.bytes <= limits.max_file_bytes
            }
            MemberKind::Directory => {
                member.bytes == 0 && member.sha256.is_empty() && member.link_target.is_empty()
            }
            MemberKind::Link => {
                member.bytes == 0 && member.sha256.is_empty() && relative(&member.link_target)
            }
        };
        need(&mut errors, valid, "member_metadata");
    }
    for path in members.keys() {
        for (offset, _) in path.match_indices('/') {
            if let Some(kind) = members.get(&path[..offset]) {
                need(
                    &mut errors,
                    matches!(kind, MemberKind::Directory),
                    "non_directory_ancestor",
                );
            }
        }
    }
    need(
        &mut errors,
        request.coverage.complete_asserted
            && request.coverage.member_count == request.members.len()
            && request.coverage.regular_file_bytes == member_total
            && member_total <= limits.max_total_read_bytes
            && request.coverage.traversal_policy == "selected_members_only"
            && request.coverage.case_collision_policy == "reject_case_folded",
        "member_coverage_assertion",
    );
    need(
        &mut errors,
        member_total
            .checked_add(part_total)
            .is_some_and(|n| n <= limits.max_total_read_bytes),
        "combined_declared_byte_limit",
    );
    let member_manifest_sha256 = member_digest(&request.members);
    need(
        &mut errors,
        request.readback.complete_asserted
            && text(&request.readback.receipt)
            && request.readback.member_manifest_sha256 == member_manifest_sha256
            && request.readback.archive_sha256 == archive.whole_sha256,
        "readback_assertion",
    );
    let retention = &request.retention;
    need(
        &mut errors,
        [
            &retention.private_access_policy_receipt,
            &retention.retention_policy_receipt,
            &retention.rollback_receipt,
        ]
        .iter()
        .all(|s| text(s))
            && !retention.dependency_receipts.is_empty()
            && retention.dependency_receipts.len() <= limits.max_roots
            && retention.dependency_receipts.iter().all(|s| text(s)),
        "retention_assertion",
    );
    let proposed_action = match request.proposed_action {
        Action::RetireMaterialization => "retire_materialization",
    };
    Plan {
        schema: "humanymous.evidence-layout-plan.v1",
        state: if errors.is_empty() {
            "proposed"
        } else {
            "refused"
        },
        structurally_valid: errors.is_empty(),
        supplied_assertions_only: true,
        candidate_filesystem_verified: false,
        terminal_handles_independently_verified: false,
        archive_bytes_verified: false,
        readback_independently_verified: false,
        owner_authority_verified: false,
        retirement_authorized: false,
        execution_authorized: false,
        proposed_member_count: request.members.len(),
        proposed_part_count: archive.parts.len(),
        proposed_member_bytes: member_total,
        proposed_archive_bytes: part_total,
        proposed_action,
        proposed_archive_format,
        proposed_handle_counts,
        member_manifest_sha256,
        errors: errors.into_iter().collect(),
        required_next_gates: [
            "owner_authority",
            "terminal_consumers_and_leases",
            "candidate_identity_and_coverage",
            "archive_bytes_and_safe_readback",
            "retention_and_explicit_disposition",
        ],
    }
}
