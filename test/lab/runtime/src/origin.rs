//! Owned loopback origin and measurable effects; never an external target.
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
pub struct Origin {
    pub address: SocketAddr,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Origin {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let observed = count.clone();
        let done = stop.clone();
        let worker = std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut bytes = Vec::new();
                        let mut part = [0; 1024];
                        loop {
                            let n = stream.read(&mut part).unwrap();
                            assert!(n > 0);
                            bytes.extend_from_slice(&part[..n]);
                            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                                let head = String::from_utf8(bytes[..end].to_vec()).unwrap();
                                let length: usize = head
                                    .lines()
                                    .filter_map(|l| l.split_once(':'))
                                    .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
                                    .unwrap()
                                    .1
                                    .trim()
                                    .parse()
                                    .unwrap();
                                if bytes.len() >= end + 4 + length {
                                    assert_eq!(&bytes[end + 4..], b"effect");
                                    assert!(!head.to_ascii_lowercase().contains("authorization:"));
                                    assert!(!head.to_ascii_lowercase().contains("x-humanymous-"));
                                    break;
                                }
                            }
                            assert!(bytes.len() < 70_000);
                        }
                        observed.fetch_add(1, Ordering::SeqCst);
                        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\nok").unwrap();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("owned origin: {e}"),
                }
            }
        });
        Self {
            address: origin,
            count,
            stop,
            worker: Some(worker),
        }
    }
    pub fn effects(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}
impl Drop for Origin {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
