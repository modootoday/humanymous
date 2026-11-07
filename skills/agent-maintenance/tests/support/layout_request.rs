use std::path::Path;

pub fn request(root: &Path) -> serde_json::Value {
    let retained = root.join(".spec.local/retained-bundle");
    serde_json::json!({
        "schema_version":1,"candidate_id":"candidate-one","repository_root":root,
        "materialization_path":root.join("old-copy"),"retained_root":retained,
        "scratch_path":root.join("fresh-scratch"),
        "owner":{"lane":"agents-meta","provider":"codex","session":"fixture-session","disposition_authority_receipt":"evidence:owner-assertion"},
        "source":{"checkpoint":"1".repeat(40),"immutable_identity_sha256":"2".repeat(64),
            "historical_executed_path":root.join("historical-copy"),"original_receipt":"evidence:historical"},
        "consumers":{"handles":[{"id":"handle-one","kind":"build_test","state":"terminal","exit_code":0,"terminal_receipt":"evidence:terminal-assertion"}],
            "inventory_complete_asserted":true,"no_remaining_consumers_asserted":true,"released_lease_receipts":["evidence:lease-assertion"]},
        "archive":{"manifest_path":retained.join("archive.json"),"format":"tar_zstd",
            "extraction_profile_receipt":"evidence:extraction-profile","whole_sha256":"b".repeat(64),"whole_bytes":7,
            "creation_receipt":"evidence:archive-assertion","parts":[{"ordinal":0,"path":retained.join("part-0000.zst"),"bytes":7,"sha256":"b".repeat(64)}]},
        "coverage":{"member_count":1,"regular_file_bytes":3,"complete_asserted":true,
            "traversal_policy":"selected_members_only","case_collision_policy":"reject_case_folded"},
        "members":[{"path":"source.bin","kind":"file","bytes":3,"sha256":"a".repeat(64),"link_target":"","mode":420}],
        "readback":{"receipt":"evidence:readback-assertion","member_manifest_sha256":"1c5bdfb5a5d8587e8609ebbccdb4a22a87cf0eb654aed76f2f748997212edda5",
            "archive_sha256":"b".repeat(64),"complete_asserted":true},
        "retention":{"private_access_policy_receipt":"evidence:private-policy","dependency_receipts":["evidence:dependencies"],
            "retention_policy_receipt":"evidence:retention","rollback_receipt":"evidence:rollback"},
        "proposed_action":"retire_materialization"
    })
}
