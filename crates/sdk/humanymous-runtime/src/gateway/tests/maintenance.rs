//! Composite sweep of the serving owner: ports of legacy internal/gate
//! gc_test.go `TestServerGCRunsWithoutPanic` and the cmd/gate one-minute
//! maintenance ticker at baseline 981d25c. The per-kernel sweeps
//! (`TestVerdictStoreGC`, `TestBanStoreGCSweepsStrikesAndExpired`,
//! `TestSweepDetectorGC`) stay with their kernels.
use super::edge::{ban_store, edge_config};
use super::*;
use crate::gateway::ban::{Key, Outcome, STRIKE_DECAY};
use std::time::{Duration, Instant};

const HOUR: Duration = Duration::from_secs(3600);

/// Flood `ip:1.2.3.4` at the store's instant zero until the hard breach
/// applies the first (one hour) auto-ban and its strike.
fn auto_banned(store: &Arc<Mutex<crate::gateway::ban::Store>>) {
    let key = Key::parse("ip:1.2.3.4").unwrap();
    let mut store = store.lock().unwrap();
    for _ in 0..64 {
        if matches!(
            store.observe(&key, Duration::ZERO).unwrap(),
            Outcome::AutoBanned { .. }
        ) {
            return;
        }
    }
    panic!("no auto-ban");
}

/// Go TestServerGCRunsWithoutPanic: the owner sweeps with no optional kernel
/// installed, and its sweep drops expired sessions.
#[test]
fn server_gc_sweeps_sessions_without_optional_kernels() {
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    app.gc(1).unwrap();
    issue(&mut app, 10_000);
    app.gc(10_001).unwrap();
    assert_eq!(app.held_sessions(), 1);
    // `session_ttl_ms` is 60 000: the session expires at 70 000.
    app.gc(70_000).unwrap();
    assert_eq!(app.held_sessions(), 0);
    assert!(app.epoch_health().serving());
}

/// The owner's sweep reaches the installed ban store: an expired auto-ban
/// drops at once and its strike survives until `STRIKE_DECAY`, as Go
/// `Server.GC` swept `BanStore.GC`.
#[test]
fn server_gc_sweeps_the_installed_ban_store() {
    let store = ban_store(8, 10);
    auto_banned(&store);
    assert_eq!(store.lock().unwrap().held(), (1, 1));
    let mut app = Application::new(
        edge_config("127.0.0.1:9".parse().unwrap()),
        Default::default(),
        Box::new(Audit::default()),
    )
    .unwrap();
    app.install_bans(store.clone(), Instant::now()).unwrap();
    app.gc_at(1, Some(HOUR / 2)).unwrap();
    assert_eq!(store.lock().unwrap().held(), (1, 1));
    app.gc_at(1, Some(2 * HOUR)).unwrap();
    assert_eq!(store.lock().unwrap().held(), (0, 1));
    app.gc_at(1, Some(STRIKE_DECAY + 2 * HOUR)).unwrap();
    assert_eq!(store.lock().unwrap().held(), (0, 0));
}

/// A sweep never moves or poisons the session epoch: a clock behind the last
/// request or past the safe range sweeps nothing and the epoch keeps serving.
#[test]
fn server_gc_never_moves_or_poisons_the_session_epoch() {
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    issue(&mut app, 10_000);
    app.gc(5_000).unwrap();
    app.gc(u64::MAX).unwrap();
    assert_eq!(app.held_sessions(), 1);
    assert!(app.epoch_health().serving());
    // The next request at the original clock is still served.
    issue(&mut app, 10_001);
    assert_eq!(app.held_sessions(), 2);
}

/// A poisoned ban store lock is reported as the ledger refusal.
#[test]
fn server_gc_reports_an_unavailable_ban_ledger() {
    let store = ban_store(8, 10);
    let mut app = Application::new(
        edge_config("127.0.0.1:9".parse().unwrap()),
        Default::default(),
        Box::new(Audit::default()),
    )
    .unwrap();
    app.install_bans(store.clone(), Instant::now()).unwrap();
    let poisoner = store.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock().unwrap();
        panic!("poison the ban store lock");
    })
    .join();
    let refused = app.gc(1).unwrap_err();
    assert_eq!((refused.status, refused.code), (503, "ban_ledger"));
}

/// The accept loop runs the sweep on its maintenance tick with no request in
/// flight (Go cmd/gate ticker calling `srv.GC`). The default period is the
/// legacy one minute; the test shortens it to observe the tick.
#[tokio::test]
async fn accept_loop_sweeps_on_the_maintenance_tick() {
    let store = ban_store(8, 10);
    let key = Key::parse("ip:192.0.2.7").unwrap();
    store
        .lock()
        .unwrap()
        .add_temporary(&key, "tick", None, Duration::from_secs(1), Duration::ZERO)
        .unwrap();
    assert_eq!(store.lock().unwrap().held(), (1, 0));
    let mut app = Application::new(
        edge_config("127.0.0.1:9".parse().unwrap()),
        Default::default(),
        Box::new(Audit::default()),
    )
    .unwrap();
    assert_eq!(app.sweep_interval(), Duration::from_secs(60));
    app.set_sweep_interval(Duration::from_millis(20));
    // The store's clock started five seconds ago, so the ban has expired; only
    // a sweep removes it because no request reaches the ban gate.
    let epoch = Instant::now().checked_sub(Duration::from_secs(5)).unwrap();
    app.install_bans(store.clone(), epoch).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = tokio::spawn(app.serve(listener));
    let deadline = Instant::now() + Duration::from_secs(10);
    while store.lock().unwrap().held() != (0, 0) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(store.lock().unwrap().held(), (0, 0));
    server.abort();
}
