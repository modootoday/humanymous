//! Ports of legacy internal/gate/ban_test.go and the validBanKey wargame rounds
//! (R051-R065, R181, R182, R199) at baseline 981d25c, plus Rust dual-control and
//! capacity boundaries.
use super::*;
use crate::network::history::rate::{Config, Limits};

const T0: Duration = Duration::from_secs(1_000_000);

fn store() -> Store {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = Config::new(Duration::from_secs(10), 3, 5, limits).unwrap();
    Store::new(rate, 16).unwrap()
}
fn key(text: &str) -> Key {
    Key::parse(text).unwrap()
}
fn flood(store: &mut Store, key: &Key, now: Duration) -> (Entry, bool) {
    let mut last = None;
    for i in 0..8 {
        if let Outcome::AutoBanned {
            entry,
            dual_control_required,
        } = store.observe(key, now + Duration::from_millis(i)).unwrap()
        {
            last = Some((entry, dual_control_required));
        }
    }
    last.expect("hard flood must auto-ban")
}

#[test]
fn auto_ban_escalates_and_permanent_tier_needs_dual_control() {
    let mut s = store();
    let k = key("ip:9.9.9.9");
    let mut now = T0;
    for (strike, term, dual) in [
        (1, HOUR, false),
        (2, 6 * HOUR, false),
        (3, 24 * HOUR, false),
        (4, 24 * HOUR, true),
    ] {
        let (entry, dual_control_required) = flood(&mut s, &k, now);
        assert_eq!(entry.strike, strike);
        assert_eq!(entry.until, Some(entry.created + term), "strike {strike}");
        assert!(entry.created >= now && entry.created < now + Duration::from_millis(8));
        assert_eq!(entry.source, Source::Auto);
        assert_eq!(dual_control_required, dual);
        assert!(
            !entry.permanent(),
            "auto escalation never creates permanent bans"
        );
        assert_eq!(s.lift(&k), Ok(true));
        now += Duration::from_secs(60);
    }
}

#[test]
fn strikes_decay_after_seven_days() {
    let mut s = store();
    let k = key("fp:abcdef");
    let (first, _) = flood(&mut s, &k, T0);
    assert_eq!(first.strike, 1);
    s.lift(&k).unwrap();
    let later = T0 + STRIKE_DECAY + Duration::from_secs(1);
    let (again, _) = flood(&mut s, &k, later);
    assert_eq!(again.strike, 1);
}

#[test]
fn already_banned_source_reports_existing_ban() {
    let mut s = store();
    let k = key("ip:9.9.9.8");
    let (entry, _) = flood(&mut s, &k, T0);
    let next = T0 + Duration::from_secs(1);
    for _ in 0..5 {
        s.observe(&k, next).unwrap();
    }
    assert_eq!(s.observe(&k, next).unwrap(), Outcome::Banned(entry));
}

#[test]
fn temporary_ban_expires() {
    let mut s = store();
    let k = key("ip:1.1.1.1");
    s.add_temporary(&k, "manual", Some("INC-1"), HOUR, T0)
        .unwrap();
    assert!(s.check(&k, T0).is_some());
    assert!(s.check(&k, T0 + Duration::from_secs(61 * 60)).is_none());
    assert!(s.list(T0).is_empty());
}

#[test]
fn list_returns_only_active_bans() {
    let mut s = store();
    s.add_temporary(&key("ip:10.0.0.1"), "a", None, HOUR, T0)
        .unwrap();
    s.add_temporary(&key("ip:10.0.0.2"), "b", None, 48 * HOUR, T0)
        .unwrap();
    s.add_temporary(&key("ip:10.0.0.3"), "c", None, Duration::from_secs(60), T0)
        .unwrap();
    let listed = s.list(T0 + Duration::from_secs(120));
    assert_eq!(listed.len(), 2);
    assert_eq!(
        listed[0].remaining(T0 + Duration::from_secs(120)),
        HOUR - Duration::from_secs(120)
    );
}

#[test]
fn single_operator_cannot_ban_permanently_or_broadly() {
    let mut s = store();
    assert_eq!(
        s.add_temporary(&key("cidr:198.51.100.0/24"), "range", None, HOUR, T0),
        Err(Refusal::DualControlRequired)
    );
    assert_eq!(
        s.add_temporary(&key("ip:1.2.3.4"), "x", None, Duration::ZERO, T0),
        Err(Refusal::Duration)
    );
    for bad in ["", "line\nbreak", &"x".repeat(257)] {
        assert_eq!(
            s.add_temporary(&key("ip:1.2.3.4"), bad, None, HOUR, T0),
            Err(Refusal::Reason)
        );
    }
}

#[test]
fn capacity_refuses_instead_of_evicting() {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = Config::new(Duration::from_secs(10), 3, 5, limits).unwrap();
    let mut s = Store::new(rate, 1).unwrap();
    s.add_temporary(&key("ip:1.0.0.1"), "a", None, HOUR, T0)
        .unwrap();
    assert_eq!(
        s.add_temporary(&key("ip:1.0.0.2"), "b", None, HOUR, T0),
        Err(Refusal::Capacity)
    );
    assert!(s.check(&key("ip:1.0.0.1"), T0).is_some());
    assert!(Store::new(rate, 0).is_err());
}

#[test]
fn ban_key_rejection_matrix_matches_go_wargame_rounds() {
    for bad in [
        "ip:",
        "fp:",
        "cidr:",
        "ip: ",
        "ip:999.999.999.999",
        "ip:1.2.3.4\n",
        "ip:1.2.3.4;drop",
        "IP:1.2.3.4",
        "ip:1.2.3.4/24",
        "cidr:not-a-cidr",
        "ip:1.1.1.1%00",
        "fp:ab",
        "fp:   ",
        "cidr:x",
        "IP:1.1.1.1",
        "",
        "x:1.2.3.4",
        "fp:has\u{7f}del",
    ] {
        assert_eq!(Key::parse(bad), Err(Refusal::Key), "{bad:?}");
    }
    for good in [
        "ip:203.0.113.199",
        "ip:2001:db8::1",
        "fp:abcd",
        "cidr:10.0.0.0/8",
        "cidr:198.51.100.0/24",
        "ip:203.0.113.182",
        "ip:::ffff:203.0.113.182",
    ] {
        assert!(Key::parse(good).is_ok(), "{good}");
    }
    let world = key("cidr:0.0.0.0/0");
    assert!(world.is_broad());
    assert!(!key("ip:1.2.3.4").is_broad());
}

#[test]
fn keys_are_canonical() {
    assert_eq!(key("ip: 1.2.3.4").as_str(), "ip:1.2.3.4");
    assert_eq!(key("ip:2001:DB8::1").as_str(), "ip:2001:db8::1");
    assert_eq!(key("cidr: 10.0.0.0/8").as_str(), "cidr:10.0.0.0/8");
    assert_ne!(
        key("ip:1.2.3.4").commitment(),
        key("ip:1.2.3.5").commitment()
    );
}

#[cfg(unix)]
mod dual_control {
    use super::*;
    use crate::audit::control::{
        Administrators, Approval, ApprovalRequest, ControlAuthority, LedgerBootstrap,
        NativeAnchorRetainer, Proposal, RetentionRequirement,
    };
    use crate::audit::keys::custody::Owner;
    use ed25519_dalek::{Signer, SigningKey};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            use std::os::unix::fs::PermissionsExt;
            let path = std::env::temp_dir().join(format!(
                "humanymous-ban-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(std::fs::canonicalize(path).unwrap())
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn scope() -> Scope {
        Scope {
            installation: "installation".into(),
            node: "node".into(),
        }
    }
    fn administrators() -> Administrators {
        Administrators::new(
            scope(),
            BTreeMap::from([
                (
                    "one".into(),
                    SigningKey::from_bytes(&[11; 32]).verifying_key().to_bytes(),
                ),
                (
                    "two".into(),
                    SigningKey::from_bytes(&[12; 32]).verifying_key().to_bytes(),
                ),
            ]),
            100,
        )
        .unwrap()
    }
    fn approvals(proposal: &Proposal) -> Vec<Approval> {
        [("one", 11u8), ("two", 12u8)]
            .iter()
            .map(|(id, seed)| Approval {
                administrator: (*id).into(),
                signature: hex::encode(
                    SigningKey::from_bytes(&[*seed; 32])
                        .sign(&proposal.signing_bytes().unwrap())
                        .to_bytes(),
                ),
            })
            .collect()
    }
    struct Fixture {
        authority: ControlAuthority,
        retainer: NativeAnchorRetainer,
        _dirs: (Dir, Dir),
    }
    impl Fixture {
        fn new() -> Self {
            let local = Dir::new();
            let independent = Dir::new();
            let authority = ControlAuthority::open(
                &local.0,
                Owner::CurrentUser,
                &[13; 32],
                administrators(),
                20,
                LedgerBootstrap::New,
            )
            .unwrap();
            let retainer = NativeAnchorRetainer::open(
                &independent.0,
                Owner::CurrentUser,
                scope(),
                authority.public_key(),
                authority.anchor(),
                true,
            )
            .unwrap();
            Self {
                authority,
                retainer,
                _dirs: (local, independent),
            }
        }
        fn permit(&mut self, action: Action, target: String) -> RetainedActionPermit {
            let proposal = self.authority.propose(action, target, 10, 100).unwrap();
            let signatures = approvals(&proposal);
            self.authority
                .authorize_retained_action(
                    ApprovalRequest {
                        proposal: &proposal,
                        approvals: &signatures,
                        now: 20,
                    },
                    &mut self.retainer,
                    RetentionRequirement::FileSynced,
                )
                .unwrap()
        }
    }

    #[test]
    fn permanent_ban_persists_and_resists_single_operator_lift() {
        let mut fixture = Fixture::new();
        let mut s = store();
        let k = key("fp:abc123");
        let permit = fixture.permit(Action::PermanentBan, k.commitment());
        let entry = s
            .add_controlled(permit, &scope(), &k, "coordinated abuse", None, T0, 20)
            .unwrap();
        assert!(entry.permanent());
        assert_eq!(entry.source, Source::DualControl);
        assert!(entry.incident.is_some());
        assert!(s.check(&k, T0 + 1000 * HOUR).is_some());
        assert_eq!(s.lift(&k), Err(Refusal::DualControlRequired));
        assert!(s.check(&k, T0 + 1000 * HOUR).is_some());
    }

    #[test]
    fn broad_ban_needs_matching_grant() {
        let mut fixture = Fixture::new();
        let mut s = store();
        let range = key("cidr:198.51.100.0/24");
        let other = key("cidr:203.0.113.0/24");
        let wrong_target = fixture.permit(Action::PermanentBan, other.commitment());
        assert!(
            s.add_controlled(wrong_target, &scope(), &range, "range", Some(HOUR), T0, 20)
                .is_err()
        );
        let wrong_action = fixture.permit(Action::KillSwitch, range.commitment());
        assert!(
            s.add_controlled(wrong_action, &scope(), &range, "range", Some(HOUR), T0, 20)
                .is_err()
        );
        assert!(s.check(&range, T0).is_none());
        let grant = fixture.permit(Action::PermanentBan, range.commitment());
        let entry = s
            .add_controlled(grant, &scope(), &range, "range", Some(HOUR), T0, 20)
            .unwrap();
        assert_eq!(entry.until, Some(T0 + HOUR));
        assert_eq!(s.lift(&range), Ok(true));
    }
}
