//! Port of legacy cmd/server/maxconns_test.go (baseline 981d25c) over a real
//! loopback listener, plus the permit bounds.
use super::*;
use std::sync::Mutex;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};

type Held = Arc<Mutex<Vec<(TcpStream, ConnectionPermit)>>>;

async fn wait_for(held: &Held, count: usize) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while held.lock().unwrap().len() < count {
        assert!(
            tokio::time::Instant::now() < deadline,
            "accept loop stalled"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// TestConnConcurrencyIsCapped (wargame R2): twelve held dialers against a
/// cap of four saturate the cap exactly; freeing one slot admits one more.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connection_concurrency_is_capped() {
    const CAP: usize = 4;
    const DIALERS: usize = 12;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let cap = ConnectionCap::new(CAP).unwrap();
    let held: Held = Arc::default();
    let server = {
        let (cap, held) = (cap.clone(), Arc::clone(&held));
        tokio::spawn(async move {
            loop {
                let Ok(permit) = cap.acquire().await else {
                    return;
                };
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                held.lock().unwrap().push((stream, permit));
            }
        })
    };
    let mut dials = Vec::with_capacity(DIALERS);
    for _ in 0..DIALERS {
        if let Ok(Ok(stream)) =
            tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(addr)).await
        {
            dials.push(stream);
        }
    }
    assert!(dials.len() > CAP, "too few dialers connected");
    wait_for(&held, CAP).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(held.lock().unwrap().len(), CAP, "cap exceeded");
    assert_eq!(cap.in_use(), CAP);

    drop(held.lock().unwrap().pop());
    wait_for(&held, CAP).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        held.lock().unwrap().len(),
        CAP,
        "back-pressure released one slot"
    );

    cap.close();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
}

#[test]
fn limit_bounds_are_refused() {
    assert_eq!(ConnectionCap::new(0).err(), Some(Refusal::Limit));
    assert_eq!(
        ConnectionCap::new(MAX_CONNECTION_CAP + 1).err(),
        Some(Refusal::Limit)
    );
    assert_eq!(
        ConnectionCap::new(MAX_CONNECTION_CAP).unwrap().limit(),
        MAX_CONNECTION_CAP
    );
}

#[test]
fn permits_are_counted_and_released_on_drop() {
    let cap = ConnectionCap::new(2).unwrap();
    let a = cap.try_acquire().unwrap().unwrap();
    let _b = cap.try_acquire().unwrap().unwrap();
    assert!(cap.try_acquire().unwrap().is_none());
    assert_eq!(cap.in_use(), 2);
    drop(a);
    assert_eq!(cap.in_use(), 1);
    assert!(cap.try_acquire().unwrap().is_some());
}

#[tokio::test]
async fn close_refuses_new_and_waiting_acquisitions() {
    let cap = ConnectionCap::new(1).unwrap();
    let live = cap.acquire().await.unwrap();
    let waiter = {
        let cap = cap.clone();
        tokio::spawn(async move { cap.acquire().await.map(|_| ()) })
    };
    tokio::task::yield_now().await;
    cap.close();
    assert_eq!(waiter.await.unwrap(), Err(Refusal::Closed));
    assert_eq!(cap.try_acquire().err(), Some(Refusal::Closed));
    drop(live);
}
