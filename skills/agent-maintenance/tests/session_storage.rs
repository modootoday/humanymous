//! Real filesystem boundary fixtures. Execute on approved hosted runners only.
use humanymous_agent_maintenance::sessions::{model::Claim, storage};
use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(mode: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "humanymous-session-{}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join(".agents/sessions/claims")).unwrap();
        fs::write(root.join(".agents/sessions/lanes.json"), serde_json::to_vec(&serde_json::json!({
            "schema_version":1,"writer_mode":mode,"lanes":[
                {"id":"one","kind":"shared","primary_paths":[]},
                {"id":"two","kind":"shared","primary_paths":[]}
            ]
        })).unwrap()).unwrap();
        fs::write(root.join(".agents/sessions/ACTIVE.md"), "# Board\n\n## Handovers\n\nImmutable previous handover\n").unwrap();
        Self(root)
    }
    fn claim(&self, lane: &str, scope: &str) -> Claim {
        Claim {lane:lane.into(),provider:"codex".into(),session:"owner".into(),
            goal:"Hosted storage fixture".into(),paths:scope.into(),
            since:"2026-11-07T00:00:00Z".into(),status:"active".into()}
    }
    fn path(&self, relative: &str) -> PathBuf { self.0.join(".agents/sessions").join(relative) }
}
impl Drop for Fixture {
    fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
}

#[test]
fn coexistence_refuses_mutation_without_creating_lock_or_claim() {
    let f=Fixture::new("legacy-coexistence");
    assert!(storage::claim(&f.0,f.claim("one","a/**")).is_err());
    assert!(!f.path(".session-tool.lock").exists());
    assert!(!f.path("claims/one.json").exists());
    assert!(!f.path("events.jsonl").exists());
}

#[test]
fn owned_release_preserves_history_and_rejects_foreign_owner() {
    let f=Fixture::new("rust-exclusive");
    storage::claim(&f.0,f.claim("one","a/**")).unwrap();
    let before=fs::read(f.path("claims/one.json")).unwrap();
    for (provider,session) in [("claude","owner"),("codex","foreign")] {
        assert!(storage::release(&f.0,"one",provider,session,"Denied release").is_err());
        assert_eq!(fs::read(f.path("claims/one.json")).unwrap(),before);
        assert!(!f.path(".session-tool.lock").exists());
    }
    storage::release(&f.0,"one","codex","owner","Owned checkpoint").unwrap();
    assert!(!f.path("claims/one.json").exists());
    assert!(!f.path(".session-tool.lock").exists());
    let board=fs::read_to_string(f.path("ACTIVE.md")).unwrap();
    assert!(board.contains("Immutable previous handover"));
    assert!(board.contains("Owned checkpoint"));
    let events:Vec<serde_json::Value>=fs::read_to_string(f.path("events.jsonl")).unwrap()
        .lines().map(|line|serde_json::from_str(line).unwrap()).collect();
    assert_eq!(events.iter().map(|v|v["action"].as_str().unwrap()).collect::<Vec<_>>(),
        ["claim-intent","claimed","release-intent","released"]);
}

#[test]
fn invalid_note_leaves_no_mutation_and_overlap_does_not_take_over() {
    let f=Fixture::new("rust-exclusive");
    storage::claim(&f.0,f.claim("one","a/**")).unwrap();
    let journal=fs::read(f.path("events.jsonl")).unwrap();
    assert!(storage::release(&f.0,"one","codex","owner","forged\nrow").is_err());
    assert!(storage::claim(&f.0,f.claim("two","a/file")).is_err());
    assert_eq!(fs::read(f.path("events.jsonl")).unwrap(),journal);
    assert!(f.path("claims/one.json").is_file());
    assert!(!f.path("claims/two.json").exists());
    assert!(!f.path(".session-tool.lock").exists());
}

#[test]
fn post_intent_board_failure_retains_claim_and_lock_for_recovery() {
    let f=Fixture::new("rust-exclusive");
    fs::write(f.path(".ACTIVE.rust-next"),"Existing interrupted board update").unwrap();
    assert!(storage::claim(&f.0,f.claim("one","a/**")).is_err());
    assert!(f.path("claims/one.json").is_file());
    assert!(f.path(".session-tool.lock").is_file());
    assert!(storage::claim(&f.0,f.claim("two","b/**")).is_err());
    assert!(!f.path("claims/two.json").exists());
    assert_eq!(fs::read_to_string(f.path(".ACTIVE.rust-next")).unwrap(),"Existing interrupted board update");
}
